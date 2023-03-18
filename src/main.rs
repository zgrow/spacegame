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
use ::tui::backend::CrosstermBackend;
use ::tui::Terminal;
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

// **MAIN
fn main() -> AppResult<()> {
	std::env::set_var("RUST_BACKTRACE", "1"); //:DEBUG: enables backtrace on program crash
    // Create an application.
    let mut eng = GameEngine::new();
    // Finish construction of the Bevy instance
	eng.app
		.insert_resource(ScheduleRunnerSettings::run_once())    // the minimal scheduler
		.insert_resource(RexAssets::new())          // REXPaint assets
		.add_plugins(MinimalPlugins)                            // see note above for list
		.update(); // we want to run it at least once to cover initial setup
	// Start the map builder and get the starting map
	let mut builder = random_builder(1);
	builder.build_map();
	let worldmap = builder.get_map();
	eng.app.insert_resource(worldmap);
	eprintln!("- worldmap inserted");//:DEBUG:
	let player_spawn = Position{x: 41, y: 25};
	eng.app.insert_resource(player_spawn);
	// Spawn the player
	// ...
    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

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
