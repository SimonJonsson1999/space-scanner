
use std::io;
use std::sync::mpsc;
use std::thread;
use std::path::PathBuf;
use ratatui::{
    Frame,
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::app::App;
use crate::events::UpdateEvent;



pub struct TUI {
    exit: bool,
    app: App,
    receiver: mpsc::Receiver<UpdateEvent>,
    input_transmitter: mpsc::Sender<UpdateEvent>,
}

impl TUI {
    pub fn new(current_dir: std::path::PathBuf) -> Self {
        let (tx, rx) = mpsc::channel::<UpdateEvent>();
        let input_transmitter = tx.clone();
        let app = App::new(current_dir, tx);
        Self {
            exit: false,
            app : app,
            receiver: rx,
            input_transmitter: input_transmitter,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()>{
        let tx = self.input_transmitter.clone();
        thread::spawn(move || {
            let _ = Self::handle_input_events(tx);

        });
        
        while !self.exit {
        let event = match self.receiver.recv() {
                Ok(event) => event,
                Err(_) => break, // sender disconnected
            };
            match event {
                UpdateEvent::Input(key_event) => self.handle_key_event(key_event)?,
                UpdateEvent::SizeCalculated {path, size} => self.handle_size_update(path, size)?,
                UpdateEvent::SizeError {path} => self.handle_size_error(path)?,
            }

            terminal.draw(|frame| self.draw(frame))?;
        }
        Ok(())
    }
    fn draw(&self, frame: &mut Frame) {
        let text = format!(
            "Current dir: {}\nCurrent depth: {}\n{}\n\n[q] Quit  [Enter] Open  [↑↓] Navigate  [Backspace] Parent",
            self.format_current_dir(),
            self.app.current_depth(),
            self.app.entries_as_string(),
        );

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .title("Space Scanner")
                    .borders(Borders::ALL)
            );

        frame.render_widget(paragraph, frame.area());
    }
    fn handle_input_events(tx: mpsc::Sender<UpdateEvent>) -> io::Result<()> {
        loop {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key_event) => {
                    if tx.send(UpdateEvent::Input(key_event)).is_err() {
                        break;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> io::Result<()> {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Enter => {
                        self.app.change_dir();
                    }
                    KeyCode::Up => self.app.move_up(),
                    KeyCode::Down => self.app.move_down(),
                    KeyCode::Char('q') => {self.exit = true},
                    KeyCode::Backspace => self.app.go_up_dir(),
                    KeyCode::Right => self.app.increase_depth(),
                    KeyCode::Left => self.app.decrease_depth(),
                    _ => {}
                }
            }
            Ok(())       
    }

    fn handle_size_update(&mut self, path: PathBuf, size: u64) -> io::Result<()> {
        self.app.update_size(&path, size);
        Ok(())
    }
    fn handle_size_error(&mut self, path: PathBuf) -> io::Result<()>  {
        self.app.set_size_error(&path);
        Ok(())
    }

    fn format_current_dir(&self) -> String{
    self
    .app
    .current_dir()
    .to_string_lossy()
    .replace("\\", " > ")
    }

    
}
