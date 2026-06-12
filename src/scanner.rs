use std::path::{Path, PathBuf};
use std::fs;
use crate::events::UpdateEvent;

#[derive(Debug, PartialEq)]
pub enum SizeState {
    Pending,
    Calculated(u64),
    Error,
}
#[derive(Debug, PartialEq)]
pub enum EntryType {
    File,
    Directory,
}

#[derive(Debug, PartialEq)]
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
        let scanner = Self {
            size_calculator,
            current_dir,
            depth,
            entries: Vec::new(),
        };
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
    pub fn navigate_to_index(&mut self, index: usize) -> bool{
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
        let entries = match std::fs::read_dir(path) {
                Ok(entries) => entries,
                Err(_) => return,
            };
        for entry in entries {
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
        let mut groups: Vec<Vec<DirectoryEntry>> = Vec::new();
        let mut current_group: Vec<DirectoryEntry> = Vec::new();
        for entry in self.entries.drain(..) {
            if !current_group.is_empty() && entry.depth == 0 {
                groups.push(std::mem::take(&mut current_group));
            }
            current_group.push(entry);
        }
        if !current_group.is_empty() {
            groups.push(std::mem::take(&mut current_group));
        }
        groups.sort_by(|a,b|{
            match (&a[0].size_state, &b[0].size_state) {
                // Largest size
                (
                SizeState::Calculated(a_size),
                SizeState::Calculated(b_size),
            ) => b_size.cmp(a_size),

            // A size is greater than error or pending
            (SizeState::Calculated(_), _) => std::cmp::Ordering::Less,
            (_, SizeState::Calculated(_)) => std::cmp::Ordering::Greater,

            // Pending is greater than error
            (SizeState::Pending, SizeState::Error) => std::cmp::Ordering::Less,
            (SizeState::Error, SizeState::Pending) => std::cmp::Ordering::Greater,

            _ => std::cmp::Ordering::Equal,
            }
        });

        self.entries = groups
                        .into_iter()
                        .flatten()
                        .collect();
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



// Unit Tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf};
    mod directory_entry {
        use super::*;
        fn make_entry(
                entry_type: EntryType,
                size_state: SizeState,)
                -> DirectoryEntry {
                DirectoryEntry {
                    name: "test".into(),
                    path: PathBuf::new(),
                    depth: 0,
                    kind: entry_type,
                    size_state,
                }
            }
        
        #[test]
        fn directory_entry_size_text() {
            let cases = vec![
                (SizeState::Pending, EntryType::File, "[Scanning]"),
                (SizeState::Pending, EntryType::Directory, "[Scanning]"),
                (SizeState::Error, EntryType::Directory, "ERR"),
                (SizeState::Error, EntryType::File, "ERR"),
                (SizeState::Calculated(512), EntryType::File, "512 B"),
                (SizeState::Calculated(10_000), EntryType::File, "9.77 KB"),
                (SizeState::Calculated(50), EntryType::Directory, "50 B"),
                (SizeState::Calculated(100_123_456_789), EntryType::Directory, "93.25 GB"),
                ];
            for (size_state, entry_type, expected) in cases {
                assert_eq!(make_entry(entry_type, size_state).size_text(), expected);
            }
        }
        #[test]
        fn directory_entry_icon() {
            let cases = vec!(
                (SizeState::Pending, EntryType::File, "📄"),
                (SizeState::Pending, EntryType::Directory, "📁"),
                (SizeState::Error, EntryType::File, "📄"),
                (SizeState::Error, EntryType::Directory, "📁"),
                (SizeState::Calculated(512), EntryType::File, "📄"),
                (SizeState::Calculated(512), EntryType::Directory, "📁"),
            );
            for (size_state, entry_type, expected) in cases {
                assert_eq!(make_entry(entry_type, size_state).icon(), expected);
            }

        }

        #[test]
        fn create_file_entry_success() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("test.txt");
            std::fs::write(&path, vec![0u8; 512]).unwrap();

            let depth = 0;
            let entry = DirectoryEntry{
                name: "test".to_string(),
                path: path.clone(),
                depth: 0,
                kind: EntryType::File,
                size_state: SizeState::Calculated(512)

            };
            let file_entry = DirectoryEntry::file("test".to_string(), path.clone(), depth);
            assert_eq!(entry, file_entry);   
        }

        #[test]
        fn create_file_entry_failure() {
            let dir = tempfile::tempdir().unwrap();
            let fake_path = dir.path().join("test_does_not_exist.txt");
            let depth= 0;
            let entry = DirectoryEntry{
                name: "test".to_string(),
                path: fake_path.clone(),
                depth: 0,
                kind: EntryType::File,
                size_state: SizeState::Error
            };
            let file_entry: DirectoryEntry = DirectoryEntry::file("test".to_string(), fake_path.clone(), depth);
            assert_eq!(entry, file_entry);
        }
        #[test]
        fn create_directory_entry() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path();
            let depth= 0;
            let entry = DirectoryEntry{
                name: "test".to_string(),
                path: path.to_path_buf(),
                depth: 0,
                kind: EntryType::Directory,
                size_state: SizeState::Pending
            };
            let file_entry: DirectoryEntry = DirectoryEntry::directory("test".to_string(), path.to_path_buf(), depth);
            assert_eq!(entry, file_entry);
        }
    }

    mod directory_scanner {

use crate::scanner::SizeState::Calculated;

use super::*;

        fn create_empty_directory_scanner() -> DirectoryScanner {
            let (tx, _) = std::sync::mpsc::channel::<UpdateEvent>();
            let size_calculator = SizeCalculator::new(tx);
            DirectoryScanner {
                size_calculator,
                current_dir:PathBuf::new(),
                depth: 0,
                entries: Vec::new()
                }
        }
        fn add_entry(directory_scanner: &mut DirectoryScanner,
            path: PathBuf,
            size_state: SizeState,
            depth: usize,
            kind: EntryType) {
                directory_scanner
                .entries
                .push(
                    DirectoryEntry {name: "test_name".into(),
                                path,
                                depth,
                                kind,
                                size_state});
        }

        #[test]
        fn navigate_to_directory() {
            let mut scanner = create_empty_directory_scanner();

            let temp = tempfile::tempdir().unwrap();
            let subdir = temp.path().join("test1");
            std::fs::create_dir(&subdir).unwrap();

            scanner.entries.push(
                DirectoryEntry::directory(
                    "test".into(),
                    subdir.clone(),
                    0,
                )
            );

            assert!(scanner.navigate_to_index(0));
            assert_eq!(scanner.get_current_dir(), &subdir);
        }
        #[test]
        fn navigate_to_nested_directory() {
            let mut scanner = create_empty_directory_scanner();

            let temp = tempfile::tempdir().unwrap();

            let subdir1 = temp.path().join("test1");
            let subdir2 = temp.path().join("test2");
            let subdir3 = temp.path().join("test3");
            std::fs::create_dir(&subdir1).unwrap();
            std::fs::create_dir(&subdir2).unwrap();
            std::fs::create_dir(&subdir3).unwrap();

            scanner.entries.push(
                DirectoryEntry::directory(
                    "test1".into(),
                    subdir1.clone(),
                    0,
                )
            );
            scanner.entries.push(
                DirectoryEntry::directory(
                    "test1".into(),
                    subdir2.clone(),
                    0,
                )
            );

            assert!(scanner.navigate_to_index(1));
            assert_eq!(scanner.get_current_dir(), &subdir2);

            scanner.entries.push(
                DirectoryEntry::directory(
                    "test2".into(),
                    subdir3.clone(),
                    0,
                )
            );

            assert!(scanner.navigate_to_index(0));
            assert_eq!(scanner.get_current_dir(), &subdir3);
        }
        #[test]
        fn navigate_to_file_returns_false() {
            let mut scanner = create_empty_directory_scanner();

            let temp = tempfile::tempdir().unwrap();

            let file_path = temp.path().join("file.txt");
            std::fs::write(&file_path, b"hello").unwrap();

            scanner.entries.push(
                DirectoryEntry::file(
                    "file".into(),
                    file_path,
                    0,
                )
            );

            assert!(!scanner.navigate_to_index(0));
            assert_eq!(scanner.get_current_dir(), &PathBuf::new());
        }

        #[test]
        fn sort() {
            let mut scanner = create_empty_directory_scanner();

            let temp = tempfile::tempdir().unwrap();

            let subdir_a = temp.path().join("test1");
            let subdir_b = temp.path().join("test2");
            let subdir_c = temp.path().join("test3");

            std::fs::create_dir(&subdir_a).unwrap();
            std::fs::create_dir(&subdir_b).unwrap();
            std::fs::create_dir(&subdir_c).unwrap();

            // test1 children
            let file1 = subdir_a.join("a1.txt");
            let file2 = subdir_a.join("a2.txt");

            std::fs::write(&file1, vec![0u8; 100]).unwrap();
            std::fs::write(&file2, vec![0u8; 200]).unwrap();

            // test2 child
            let file3 = subdir_b.join("b1.txt");

            std::fs::write(&file3, vec![0u8; 500]).unwrap();

            add_entry(&mut scanner, subdir_a.clone(), Calculated(300), 0, EntryType::Directory);
            add_entry(&mut scanner, file1.clone(), Calculated(200), 1, EntryType::File);
            add_entry(&mut scanner, file2.clone(), Calculated(100), 1, EntryType::File);
            add_entry(&mut scanner, subdir_b.clone(), Calculated(500), 0, EntryType::Directory);
            add_entry(&mut scanner, file3.clone(), Calculated(500), 1, EntryType::File);
            add_entry(&mut scanner, subdir_c.clone(), Calculated(0), 0, EntryType::Directory);
            scanner.sort_entries();
            assert_eq!(scanner.entries[0].path, subdir_b.clone());
            assert_eq!(scanner.entries[1].path, file3.clone());
            assert_eq!(scanner.entries[2].path, subdir_a.clone());
            assert_eq!(scanner.entries[3].path, file1.clone());
            assert_eq!(scanner.entries[4].path, file2.clone());
            assert_eq!(scanner.entries[5].path, subdir_c.clone())
        }



    }
}






