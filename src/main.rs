
mod tui;
mod scanner;
mod app;
mod events;
use std::io;
fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut tui = tui::TUI::new(std::env::current_dir()?);

    let tui_result = tui.run(&mut terminal);
    ratatui::restore();
    tui_result

}