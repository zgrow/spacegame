// main.rs
// generated from orhun/rust-tui-template via cargo-generate
// Mar 15 2023
/*  Bevy's MinimalPlugins list:
 *  - TaskPoolPlugin: task pools for handling internal system calls and similar
 *  - TypeRegistrationPlugin: registration of default types to the TypeRegistry resource
 *  - FrameCountPlugin: adds frame counting functionality
 *  - TimePlugin: adds time fxns (both system clock +events and internal timing)
 *  - ScheduleRunnerPlugin: (not in Default) handles execution of the bevy app's Schedule
 *  This is the set of required plugins for a *headless* Bevy setup, and includes an internal
 *  ScheduleRunner that would otherwise be driven by the window system's event feedback
 */
// TODO: checkout github/zkat/big-brain for a Bevy-based AI model !!! - thanx Bevy dev
// **EXTERNAL LIBS
use std::io;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use bevy::prelude::*;
use bevy::app::ScheduleRunnerSettings;

// **INTERNAL LIBS
use spacegame::app::*;
use spacegame::app::tui::Tui;
use spacegame::app::handler::key_parser;
use spacegame::app::event::{Event, EventHandler};
use spacegame::app::messagelog::MessageLog;
use spacegame::components::*;
use spacegame::rex_assets::RexAssets;
use spacegame::map_builders::random_builder;
use spacegame::camera_system::camera_update_sys;
use spacegame::sys::*;

// **MAIN
fn main() -> AppResult<()> {
	std::env::set_var("RUST_BACKTRACE", "1"); //:DEBUG: enables backtrace on program crash
	// Set up ratatui
	let backend = CrosstermBackend::new(io::stdout());
	let terminal = Terminal::new(backend)?;
	// Now that we have a terminal, check the size to make sure we can continue
	let tsize = terminal.size().unwrap();
	if tsize.width < 80 || tsize.height < 40 {
		// throw a bigtime error and bailout if the terminal is too small
		eprintln!("Terminal size: {}, {}", tsize.width, tsize.height);
		return Err("Terminal size is too small (must be 80x40 minimum)".into());
	}
	// Finish setup of ratatui
	let events = EventHandler::new(250);
	let mut tui = Tui::new(terminal, events);
	tui.init()?;
	// Set the initial list of comms channels
	let chanlist = vec!["world".to_string(),
	                    "planq".to_string(),
	                    "debug".to_string()];
	// Build up the Bevy instance
	let mut eng = GameEngine::new(tsize);
	eng.app
		.insert_resource(ScheduleRunnerSettings::run_once())
		.insert_resource(RexAssets::new())
		.insert_resource(Position{x: 35, y: 20}) // The player's position/starting spawn point
		.insert_resource(Events::<TuiEvent>::default()) // The Bevy handler for inter-system comms
		.insert_resource(MessageLog::new(chanlist))
		.add_plugins(MinimalPlugins) // see above for list of what this includes
		.add_event::<crossterm::event::KeyEvent>()
		.add_startup_system(new_player_system) // depends on having player_spawn inserted prior
		.add_startup_system(new_lmr_system)
		.add_system(movement_system)
		.add_system(visibility_system)
		.add_system(camera_update_sys);
	// Build the game world
	// TODO: i thought this was loading via bracket-rex but it has to go after the insert_resource
	// via Bevy??? need to reexamine later
	let mut builder = random_builder(1);
	builder.build_map();
	let worldmap = builder.get_map();
	eng.app.insert_resource(worldmap);
	// Build the main camera view
	eng.calc_layout(tsize);
	let main_camera = CameraView::new(eng.ui_grid[0].width as i32, eng.ui_grid[0].height as i32);
	eng.app.insert_resource(main_camera);
	// Run an initial cycle of Bevy; triggers all of the startup systems; should be last setup oper
	eng.app.update();
	// Start the main loop.
	while eng.running {
		// Render the user interface.
		tui.draw(&mut eng)?; // see tui::terminal::draw(); single cycle of tui-rs loop
		// Handle input events.
		match tui.events.next()? {
			Event::Tick => eng.tick(), // ie run an update cycle of the game state
			Event::Key(key_event) => key_parser(key_event, &mut eng)?,
			Event::Mouse(_) => {} // TODO: no mouse support yet
			Event::Resize(_, _) => {} // TODO: no resize support yet
		}
		// Update the game world
		eng.app.update();
	}
	// Exit the user interface.
	tui.exit()?;
	Ok(())
}

// EOF
