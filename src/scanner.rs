
use std::path::{Path, PathBuf};
use std::fs;
use crate::events::UpdateEvent;

pub enum SizeState {
    Pending,
    Calculated(u64),
    Error,
}
pub enum EntryType {
    File,
    Directory,
}

pub struct DirectoryEntry {
    pub name: String,
    pub path: PathBuf,
    pub depth: usize,
    pub kind: EntryType,
    pub size_state: SizeState
}
impl DirectoryEntry {
    pub fn file(
        name: String,
        path: PathBuf,
        depth: usize,
    ) -> Self {
        let size_state = match std::fs::metadata(&path) {
            Ok(metadata) => SizeState::Calculated(metadata.len()),
            Err(_) => SizeState::Error,
        };

        Self {
            name,
            path,
            depth,
            kind: EntryType::File,
            size_state,
        }
    }

    pub fn directory(
        name: String,
        path: PathBuf,
        depth: usize,
    ) -> Self {
        Self {
            name,
            path,
            depth,
            kind: EntryType::Directory,
            size_state: SizeState::Pending,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self.kind {
            EntryType::Directory => "📁",
            EntryType::File => "📄",
        }
    }

    pub fn is_dir(&self) -> bool {
        match self.kind {
            EntryType::Directory => true,
            _ => false,
        }
    }
    pub fn set_size(&mut self, size: u64) {
        self.size_state = SizeState::Calculated(size);
    }
    pub fn set_error_size(&mut self) {
        self.size_state = SizeState::Error;
    }
    pub fn size_text(&self) -> String {
        match self.size_state {
            SizeState::Pending => "[Scanning]".to_string(),
            SizeState::Calculated(size) => {
                let bytes = size as f64;
                const KB: f64 = 1024.0;
                const MB: f64 = KB * 1024.0;
                const GB: f64 = MB * 1024.0;

                if bytes >= GB {
                    format!("{:.2} GB", bytes / GB)
                } else if bytes >= MB {
                    format!("{:.2} MB", bytes / MB)
                } else if bytes >= KB {
                    format!("{:.2} KB", bytes / KB)
                } else {
                    format!("{} B", bytes as u64)
                }
            },
            SizeState::Error => "ERR".to_string(),
        }
    }
}

pub struct DirectoryScanner{
    size_calculator: SizeCalculator,
    current_dir: PathBuf,
    depth: usize,
    entries: Vec<DirectoryEntry>,
}

impl DirectoryScanner {
    pub fn new(current_dir: PathBuf, size_calculator: SizeCalculator ,depth: usize) -> Self {
        let mut scanner = Self {
            size_calculator,
            current_dir,
            depth,
            entries: Vec::new(),
        };
        scanner.update_entries();
        scanner
    }
    pub fn entries(&self) -> &[DirectoryEntry] {
        &self.entries
    }
    pub fn increase_depth(&mut self) {
        self.depth += 1
    }

    pub fn decrease_depth(&mut self) {
        self.depth -= 1
    }
    pub fn get_depth(&self) -> usize{
        self.depth
    }
    pub fn get_current_dir(&self) -> &PathBuf {
        &self.current_dir
    }
    pub fn get_parent_dir(&self) -> Option<&Path> {
        self.current_dir.parent()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn set_dir_to_index(&mut self, index: usize) -> bool{
        if let Some(entry) = self.entries.get(index) {
            if entry.is_dir() {
                self.change_dir(entry.path.clone());
                return true
            }
        }
        false
    }
    fn find_entry(&mut self, path: &PathBuf) -> Option<&mut DirectoryEntry> {
        self.entries
        .iter_mut()
        .find(|entry| entry.path == *path)
    }

    pub fn set_error_size(&mut self, path: &PathBuf) {
        if let Some(entry) = self.find_entry(path) {
            entry.set_error_size();
        }
    }
    pub fn update_size(&mut self, path: &PathBuf, size: u64) {
        if let Some(entry) = self.find_entry(path) {
            entry.set_size(size);
        }
    }
    pub fn change_dir(&mut self, path: PathBuf) {
        self.current_dir = path;
        self.update_entries();
    }

    fn traverse(&mut self, path: &PathBuf, current_depth: usize) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let entry = if path.is_dir() {
                DirectoryEntry::directory(name, path.clone(), current_depth)
            } else {
                DirectoryEntry::file(name, path.clone(), current_depth)
            };
            self.entries.push(entry);
            if path.is_dir() {
                self.size_calculator.calculate_directory_size(&path);
                if current_depth < self.depth{
                    self.traverse(&path,current_depth + 1);
                }   
            };
        }
    }

    pub fn update_entries(&mut self) {
        self.entries.clear();
        let dir = self.current_dir.clone();
        self.traverse(&dir, 0);
    }

    pub fn sort_entries(&mut self) {
        self.entries.sort_by(|a, b| {
        if a.depth != 0 || b.depth != 0 {
            return std::cmp::Ordering::Equal;
        }
        match (&a.size_state, &b.size_state) {
            (
                SizeState::Calculated(a_size),
                SizeState::Calculated(b_size),
            ) => b_size.cmp(a_size),

            (SizeState::Calculated(_), _) => std::cmp::Ordering::Less,
            (_, SizeState::Calculated(_)) => std::cmp::Ordering::Greater,

            (SizeState::Pending, SizeState::Error) => std::cmp::Ordering::Less,
            (SizeState::Error, SizeState::Pending) => std::cmp::Ordering::Greater,

            _ => std::cmp::Ordering::Equal,
        }
    });
    }

   


}

pub struct SizeCalculator {
    transmitter: std::sync::mpsc::Sender<UpdateEvent>,
}

impl SizeCalculator {
    pub fn new(transmitter: std::sync::mpsc::Sender<UpdateEvent>) -> Self {
        Self {
            transmitter,
        }
    }

    fn calculate_directory_size(&self, path: &PathBuf) {
        let tx = self.transmitter.clone();
        let dir_path = path.clone();

        std::thread::spawn(move || {
            let size: Result<u64, ()> = Self::directory_size(&dir_path);
            match size {
                Ok(size) => {
                let _ = tx.send(UpdateEvent::SizeCalculated {path: dir_path, size});
                },
                Err(_) => {
                let _ = tx.send(UpdateEvent::SizeError {path: dir_path});
                }
            }
            
        });
    }

     fn directory_size(path: &Path) -> Result<u64, ()> {
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return Err(()),
        };

        let mut size = 0;
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                size += Self::directory_size(&path)?;
            }
             else if path.is_file() {
                if let Ok(metadata) = fs::metadata(&path) {
                    size += metadata.len();
                }
            }
        }

        Ok(size)
    }
}










