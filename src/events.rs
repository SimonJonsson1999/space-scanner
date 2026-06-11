use std::path::PathBuf;
use crossterm::event::KeyEvent;
pub enum UpdateEvent {
    Input(KeyEvent),

    SizeCalculated {
        path: PathBuf,
        size: u64,
    },

    SizeError {
        path: PathBuf,
    },
}