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
// TODO: checkout github/imsnif/diskonaut for a demo of flexible ratatui box-drawing
// **EXTERNAL LIBS
use std::io;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use bevy::prelude::*;
use bevy::app::ScheduleRunnerSettings;
use bevy_save::prelude::*;

// **INTERNAL LIBS
use spacegame::app::*;
use spacegame::app::tui::Tui;
use spacegame::app::handler::key_parser;
use spacegame::app::tui_event::{Event, EventHandler};
use spacegame::app::messagelog::MessageLog;
use spacegame::app::planq::*;
use spacegame::app::event::*;
use spacegame::components::*;
use spacegame::rex_assets::RexAssets;
use spacegame::map::Model;
use spacegame::map_builders::get_builder;
use spacegame::camera_system::{camera_update_sys, CameraView};
use spacegame::sys::*;

// **MAIN
fn main() -> AppResult<()> {
	std::env::set_var("RUST_BACKTRACE", "1"); // DEBUG: enables backtrace on program crash
	// Set up ratatui
	let backend = CrosstermBackend::new(io::stdout());
	let terminal = Terminal::new(backend)?;
	// Now that we have a terminal, check the size to make sure we can continue
	let tsize = terminal.size().unwrap();
	if tsize.width < 80 || tsize.height < 40 {
		// throw a bigtime error and bailout if the terminal is too small
		eprintln!("Terminal size: {}, {}", tsize.width, tsize.height); // DEBUG:
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
		.register_saveable::<Player>()
		.register_saveable::<Planq>()
		.register_saveable::<spacegame::components::Name>() // needs full pathname
		.register_saveable::<Position>()
		.register_saveable::<Renderable>()
		.register_saveable::<Mobile>()
		.register_saveable::<Obstructive>()
		.register_saveable::<Portable>()
		.register_saveable::<Openable>()
		.register_saveable::<Lockable>()
		.register_saveable::<Container>()
		.insert_resource(GameSettings::new())
		.insert_resource(ScheduleRunnerSettings::run_once())
		.insert_resource(RexAssets::new())
		.insert_resource(Position{x: 35, y: 20, z: 0}) // The player's position/starting spawn point
		.insert_resource(Events::<GameEvent>::default()) // The Bevy handler for inter-system comms
		.insert_resource(Events::<PlanqEvent>::default())
		.insert_resource(MessageLog::new(chanlist))
		.insert_resource(PlanqData::new())
		.add_plugins(MinimalPlugins) // see above for list of what this includes
		.add_event::<crossterm::event::KeyEvent>()
		.add_startup_system(new_player_spawn) // depends on having player_spawn inserted prior
		.add_startup_system(new_planq_spawn)
		.add_startup_system(new_lmr_spawn)
		.add_system(engine_system)
		.add_system(map_indexing_system)
		.add_system(movement_system)
		.add_system(visibility_system)
		.add_system(camera_update_sys)
		.add_system(item_collection_system)
		.add_system(openable_system)
		.add_system(lock_system)
		.add_system(operable_system)
		.add_system(planq_system)
	;
	// Build the game world
	// TODO: i thought this was loading via bracket-rex but it has to go after the insert_resource
	// via Bevy??? need to reexamine later
	let mut model = Model::default();
	let cur_floor = 0;
	let mut builder = get_builder(1);
	builder.build_map();
	let mut worldmap = builder.get_map();
	// build all of the furniture, backdrops, and so on for this level
	let mut item_spawns = builder.get_item_spawn_list();
	eprintln!("item_spawns.len: {}", item_spawns.len()); // DEBUG:
	eng.artificer.spawn_batch(&mut eng.app.world, &mut item_spawns, cur_floor);
	model.levels.push(worldmap);
	// build the dev map and drop a portal to it
	builder = get_builder(99); // produces the dev map
	builder.build_map();
	//cur_floor += 1;
	worldmap = builder.get_map();
	model.levels.push(worldmap);
	model.add_portal((3, 20, 0), (5, 5, 1), true);
	// Add the game world to the engine
	eng.app.insert_resource(model); 
	// Build the main camera view
	eng.calc_layout(tsize);
	let main_camera = CameraView::new(eng.ui_grid.camera_main.width as i32, eng.ui_grid.camera_main.height as i32);
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
	}
	// Exit the user interface.
	tui.exit()?;
	Ok(())
}

// EOF
