// main.rs
// created: July 12 2023

// *** EXTERNAL LIBS
use std::io;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

// *** INTERNAL LIBS
use spacegame::engine::{
	AppResult,
	GameEngine,
	handler::key_parser,
	menu::*,
	tui::*,
	tui::Event, // this line is required for disambiguiation vs Bevy
};

// *** MAIN METHOD
fn main() -> AppResult<()> {
	std::env::set_var("RUST_BACKTRACE", "1"); // DEBUG: enables backtrace on program crash
	// Set up ratatui
	let backend = CrosstermBackend::new(io::stdout());
	let terminal = Terminal::new(backend)?;
	// Now that we have a terminal, check the size to make sure we can continue
	let tsize = terminal.size().unwrap();
	if tsize.width < 80 || tsize.height < 40 {
		// throw a bigtime error and bailout if the terminal is too small
		return Err(format!("Terminal dimensions are too small: {}x{} (80x40 min)", tsize.width, tsize.height).into());
	}
	// Finish setup of ratatui
	let events = EventHandler::new(250);
	let mut tui = Tui::new(terminal, events);
	tui.init()?;
	// Set up the game engine
	let mut eng = GameEngine::new(tsize);
	// Start the game loop
	eng.running = true;
	eng.set_menu(MenuType::Main, (30, 15));
	while eng.running {
		// Render the game interface and contents
		tui.draw(&mut eng)?;
		// Handle input events
		match tui.events.next()? {
			Event::Tick           => eng.tick(),
			Event::Key(key_event) => key_parser(key_event, &mut eng)?,
			Event::Mouse(_)       => { }
			Event::Resize(_, _)   => { }
		}
	}
	// The game loop has stopped, so exit the program
	tui.exit()?;
	Ok(())
}

// EOF
