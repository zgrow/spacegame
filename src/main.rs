// main.rs
// created: July 12 2023

// *** EXTERNAL LIBS
use std::io;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
extern crate simplelog;

use simplelog::*;

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
	// HINT: Set the LevelFilter below to change how much logging you wish to see
	// NOTE: Don't enable the Trace level filter for the logger unless you want a LOT of thread-level feedback
	let _ = TermLogger::init(LevelFilter::Debug, Config::default(), TerminalMode::Stderr, ColorChoice::Auto);
	//error!("This is a test error message"); // Level::Error
	//warn!("This is a test warn message"); // Level::Warn
	//info!("This is a test info message"); // Level:: Info
	//debug!("This is a test debug message"); // Level::Debug, will include some debug context info prepended to the message
	//trace!("This is a test trace message"); // Level::Trace, will include any trace debug info from other modules that support it!
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
