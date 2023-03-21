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
// **EXTERNAL LIBS
use std::io;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use ratatui::layout::{Layout, Constraint, Direction, Rect};
use bevy::prelude::*;
use bevy::app::ScheduleRunnerSettings;

// **INTERNAL LIBS
use spacegame::app::*;
use spacegame::app::tui::Tui;
use spacegame::app::handler::handle_key_events;
use spacegame::app::event::{Event, EventHandler};
use spacegame::components::*;
use spacegame::rex_assets::RexAssets;
use spacegame::map_builders::random_builder;
use spacegame::camera_system::camera_update_sys;

// **MAIN
fn main() -> AppResult<()> {
	std::env::set_var("RUST_BACKTRACE", "1"); //:DEBUG: enables backtrace on program crash
	// Create an application.
	// Initialize the terminal user interface.
	let backend = CrosstermBackend::new(io::stdout());
	let terminal = Terminal::new(backend)?;
	// Now that we have a terminal, check the size to make sure we can continue
	let tsize = terminal.size().unwrap();
	//eprintln!("{} {}", tsize.width, tsize.height);//:DEBUG:
	if tsize.width < 80 || tsize.height < 60 {
		// throw a bigtime error and bailout
		return Err("ERROR: Terminal size is too small (must be 80x60 minimum)".into());
	}
	// Precompute the sizes of the layout elements in the UI
	// main_grid[index]: 0 = CameraView, 1 = MessageLog, 2 = MonitorRack
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
	// NOTE: the layout grid can't go into the ECS because tui::Rect does not have Resource
	let main_camera = CameraView::new(main_grid[0].width as i32, main_grid[0].height as i32);//:FIXME: magic nums
	// Finish setup of ratatui
	let events = EventHandler::new(250);
	let mut tui = Tui::new(terminal, events);
	tui.init()?;
	// Build up the Bevy instance
	let mut eng = GameEngine::new(main_grid);
	eng.app
		.insert_resource(ScheduleRunnerSettings::run_once())    // the minimal scheduler
		.insert_resource(RexAssets::new())          // REXPaint assets
		.add_plugins(MinimalPlugins)                            // see note above for list
		.add_system(camera_update_sys);
	// Register our various resources and other assets
	eng.app.insert_resource(main_camera);
	// Start the map builder and get the starting map
	let mut builder = random_builder(1);
	builder.build_map();
	let worldmap = builder.get_map();
	eng.app.insert_resource(worldmap);
	//eprintln!("- worldmap inserted");//:DEBUG:
	// Spawn the player
	let player_spawn = Position{x: 41, y: 25};
	eng.app.insert_resource(player_spawn);
	// ...

	eng.app.update(); // DO LAST: Let Bevy run a processing cycle to get things started
	// Start the main loop.
	while eng.running {
		// Render the user interface.
		tui.draw(&mut eng)?; // see tui::terminal::draw(); single cycle of tui-rs loop
		// Handle events.
		match tui.events.next()? {
			Event::Tick => eng.tick(), // ie run an update cycle of the game state
			Event::Key(key_event) => handle_key_events(key_event, &mut eng)?, // app::handle_key_events()
			Event::Mouse(_) => {} //:FIXME:
			Event::Resize(_, _) => {} //:FIXME:
		}
		eng.app.update();
	}

	// Exit the user interface.
	tui.exit()?;
	Ok(())
}

// EOF
