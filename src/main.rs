// main.rs
// generated from orhun/rust-tui-template via cargo-generate
// Mar 15 2023
// **EXTERNAL LIBS
use std::io;
// **INTERNAL LIBS
use spacegame::gui::{App, AppResult};
use spacegame::gui::tui::Tui;
use spacegame::gui::event::{Event, EventHandler};
use spacegame::gui::handler::handle_key_events;
use ::tui::backend::CrosstermBackend;
use ::tui::Terminal;

// **MAIN
fn main() -> AppResult<()> {
    // Create an application.
    let mut app = App::new();

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}

// EOF
