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
// NOTE: checkout github/zkat/big-brain for a Bevy-based AI model !!! - thanx Bevy dev
// **EXTERNAL LIBS
use std::io;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use ratatui::layout::{Layout, Constraint, Direction};
use bevy::prelude::*;
use bevy::app::ScheduleRunnerSettings;

// **INTERNAL LIBS
use spacegame::app::*;
use spacegame::app::tui::Tui;
use spacegame::app::handler::key_parser;
use spacegame::app::event::{Event, EventHandler};
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
	// Precompute the sizes of the layout elements in the UI
	// FIXME: This logic is duplicated in app/mod.rs! Changes here need to be there too
	// REFER: main_grid[index]: 0 = CameraView, 1 = MessageLog, 2 = MonitorRack
	// Calculate the left-right split
	let mut big_split = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([Constraint::Min(30), Constraint::Length(20)].as_ref())
		.split(tsize).to_vec();
	// Calculate the camera/message split
	let mut main_grid = Layout::default()
		.direction(Direction::Vertical)
		.constraints([Constraint::Min(30), Constraint::Length(20)].as_ref())
		.split(big_split[0]).to_vec();
	// Attach the splits in order (see above)
	main_grid.push(big_split.pop().unwrap());
	let main_camera = CameraView::new(main_grid[0].width as i32, main_grid[0].height as i32);//:FIXME: magic nums
	// Finish setup of ratatui
	let events = EventHandler::new(250);
	let mut tui = Tui::new(terminal, events);
	tui.init()?;
	// Set up the Bevy event handler for inter-system comms
	let tui_events = Events::<TuiEvent>::default();
	// Spawn the player
	let player_spawn = Position{x: 35, y: 20};
	// FIXME: this is where creation of the player entity will go
	// Build up the Bevy instance
	let mut eng = GameEngine::new(main_grid);
	eng.app
		.insert_resource(ScheduleRunnerSettings::run_once())
		.insert_resource(RexAssets::new())
		.insert_resource(main_camera)
		//.insert_resource(worldmap)
		.insert_resource(player_spawn)
		.insert_resource(tui_events)
		.add_plugins(MinimalPlugins) // see above for list of what this includes
		.add_event::<crossterm::event::KeyEvent>()
		.add_startup_system(new_player_system) // depends on having player_spawn inserted prior
		.add_system(movement_system)
		.add_system(camera_update_sys);
		// First Bevy cycle should fire all of the startup systems, so make sure this iš LAST
		//.update();
	// Build the game world - i thought this was loading via bracket-rex but it has to go after the insert_resource via Bevy??? need to reexamine later
	let mut builder = random_builder(1);
	builder.build_map();
	let worldmap = builder.get_map();
	eng.app.insert_resource(worldmap);
	eng.app.update();//:DEBUG: must be last before starting game
	// Start the main loop.
	while eng.running {
		// Render the user interface.
		tui.draw(&mut eng)?; // see tui::terminal::draw(); single cycle of tui-rs loop
		// Handle input events.
		match tui.events.next()? {
			Event::Tick => eng.tick(), // ie run an update cycle of the game state
			//Event::Key(key_event) => handle_key_events(key_event, &mut eng, &mut events)?, // app::handle_key_events()
			Event::Key(key_event) => key_parser(key_event, &mut eng)?,
			Event::Mouse(_) => {} //:FIXME:
			Event::Resize(_, _) => {} //:FIXME:
		}
		// Handle game events
		//events.update();
		// Update the game world
		eng.app.update();
	}
	// Exit the user interface.
	tui.exit()?;
	Ok(())
}

// EOF
