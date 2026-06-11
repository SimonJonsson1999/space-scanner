
use std::path::PathBuf;
use std::sync::mpsc;
use crate::scanner::{DirectoryScanner, SizeCalculator};
use crate::events::UpdateEvent;

pub struct App {
    pub selected: usize,
    pub directory_scanner: DirectoryScanner,
    pub max_depth: usize,
}

impl App {
    pub fn new(current_dir: PathBuf, transmitter: mpsc::Sender<UpdateEvent>) -> Self {
        let size_calculator = SizeCalculator::new(transmitter);
        let mut directory_scanner = DirectoryScanner::new(current_dir, size_calculator,0);
        directory_scanner.update_entries();
        Self {
            selected: 0,
            directory_scanner,
            max_depth: 5
        }
    }

    pub fn entries_as_string(&self) -> String {
        let mut output: String = String::new();

        for (index, entry) in self.directory_scanner.entries().iter().enumerate() {
            let marker = if index == self.selected { ">" } else { " " };
            let icon: &str = entry.icon();
            let indent = "  ".repeat(entry.depth);
            output.push_str(&format!(
                "{}{} {} {:<40} {:>10}\n",
                marker,
                indent,
                icon,
                entry.name,
                entry.size_text(),
            ));

        }
        output
    }
    pub fn move_down(&mut self) {
        if self.selected + 1 < self.directory_scanner.len() {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
    pub fn increase_depth(&mut self) {
        if self.directory_scanner.get_depth() < self.max_depth {
            self.directory_scanner.increase_depth();
            self.directory_scanner.update_entries();
            self.directory_scanner.sort_entries();
            if self.selected > self.directory_scanner.len() {
                self.selected = 0;
            }

        }
    }
    
    pub fn decrease_depth(&mut self) {
        if self.directory_scanner.get_depth() > 0 {
            self.directory_scanner.decrease_depth();
            self.directory_scanner.update_entries();
            self.directory_scanner.sort_entries();
            if self.selected > self.directory_scanner.len() {
                self.selected = 0;
            }
        }
       
    }

    pub fn change_dir(&mut self) {
        if self.directory_scanner.navigate_to_index(self.selected) {
            self.selected = 0;
        }     
}
    pub fn go_up_dir(&mut self) {
        if let Some(parent) = self.directory_scanner.get_parent_dir() {
            self.directory_scanner.change_dir(parent.to_path_buf());
            self.directory_scanner.update_entries();
            self.selected = 0;
        }
    }
    pub fn update_size(&mut self, path: &PathBuf, size: u64) {
        self.directory_scanner.update_size(path, size);
        self.directory_scanner.sort_entries();
    }
    pub fn set_size_error(&mut self, path: &PathBuf) {
        self.directory_scanner.set_error_size(path);
    }

    pub fn current_dir(&self) -> &PathBuf {
        self.directory_scanner.get_current_dir()
    }
    pub fn current_depth(&self) -> usize {
        self.directory_scanner.get_depth()
    }

}