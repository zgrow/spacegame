// engine/mod.rs
// July 12 2023

// *** EXTERNAL LIBS
use std::borrow::Cow;
use std::error;
use bevy::{
	prelude::*,
	utils::*,
};
use bevy_save::prelude::*;
use bevy_turborand::prelude::*;
use bracket_rex::prelude::*;
use ratatui::{
	Frame,
	backend::Backend,
	layout::{
		Constraint,
		Direction,
		Layout,
		Rect
	},
	style::{
		Color,
		Style
	},
	widgets::*,
};
use strum::IntoEnumIterator;

// *** INTERNAL LIBS
pub mod event;
pub mod handler;
pub mod image_loader;
pub mod menu;
pub mod messagelog;
pub mod planq;
pub mod tui;
pub mod viewport;
use crate::{
	artisan::*,
	camera::*,
	components::*,
	engine::{
		event::*,
		image_loader::load_rex_pgraph,
		menu::*,
		messagelog::*,
		planq::*,
		viewport::*
	},
	map::*,
	mason::{
		get_builder,
		MapBuilder,
	},
	rex_assets::*,
	sys::*
};

// *** GameEngine
pub struct GameEngine<'a> {
	pub running:        bool, // If true, the game loop is running
	pub standby:        bool, // If true, the game loop is on standby (ie paused)
	pub mode:           EngineMode,
	pub bevy:           App, // bevy::app::App, contains all of the ECS and related things
	pub mason:          Box<dyn MapBuilder>,
	pub artisan:        ItemBuilder,
	pub visible_menu:   MenuType,
	pub menu_main:      MenuState<Cow<'static, str>>,
	pub menu_actions:   MenuState<ActionType>,
	pub menu_entities:  MenuState<Entity>,
	pub menu_context:   MenuState<GameEvent>,
	pub menu_posn:      (u16, u16),
	pub ui_grid:        UIGrid,
	pub layout_changed: bool,
	pub default_block:  Block<'a>,
	pub default_style:  Style,
	pub savegame_filename: String,
	pub term_dims:      Rect,
	pub planq_stdin: PlanqInput<'a>,
}
impl GameEngine<'_> {
	/// Constructs a new instance of [`GameEngine`].
	pub fn new(max_area: Rect) -> Self {
		let mut new_eng = GameEngine {
			running: false,
			standby: true,
			mode: EngineMode::Standby,
			bevy: App::new(),
			mason: get_builder(1), // WARN: only pulls in the DevMapBuilder right now
			artisan: ItemBuilder::new(),
			// HINT: These menu items are handled via a match case in GameEngine::tick()
			visible_menu: MenuType::None,
			menu_main: MenuState::new(vec![]),
			menu_actions: MenuState::new(vec![]),
			menu_entities: MenuState::new(vec![]),
			menu_context: MenuState::new(vec![]),
			menu_posn: (0, 0),
			ui_grid: UIGrid::new(),
			layout_changed: true,
			default_block: Block::default().borders(Borders::ALL).border_type(BorderType::Plain),
			default_style: Style::default().fg(Color::White).bg(Color::Black),
			savegame_filename: "demo_game".to_string(),
			term_dims: max_area,
			planq_stdin: PlanqInput::new(),
		};
		new_eng.bevy.add_plugins(MinimalPlugins).add_plugins(SavePlugins);
		new_eng
	}
	/// Runs a single update cycle of the GameEngine
	pub fn tick(&mut self) {
		/* HINT: This is a known-good local method for obtaining data from a selected entity
		_ => {
			eprintln!("! unhandled option '{}' selected from menu", item); // DEBUG: report an unhandled menu option
			let enty_id = item.parse::<u32>().unwrap();
			let enty_ref = self.bevy.world.entities().resolve_from_id(enty_id);
			if let Some(enty) = enty_ref {
				if self.bevy.world.entities().contains(enty) {
					eprintln!("* produced a valid enty_ref from an entity.index()"); // DEBUG: report entity reference success
				if let Some(name) = self.bevy.world.get::<ActorName>(enty) {
						eprintln!("* Entity {} named {} was selected", enty_id, name.name.clone()); // DEBUG: announce entity selection
					} else {
						eprintln!("* Could not retrieve the name of the selected entity"); // DEBUG: report entity component retrieval failure
					}
				}
			}
		}
		*/
		// This is where I'd pull any mode changes that might have happened during the last Bevy update and apply them
		//if settings.mode_changed { ... }
		// If there are any menu events, handle them
		for event in self.menu_main.drain_events() {
			// NOTE: if the user selects a submenu heading as their choice, *nothing* will be generated; the menu will just close
			//       not sure yet if there's a way to trap that outcome
			match event {
				MenuEvent::Selected(item) => match item.as_ref() {
					"main.new_game"  => { self.new_game(); }
					"main.load_game" => { self.load_game(self.savegame_filename.clone()); }
					"main.save_game" => { self.save_game(self.savegame_filename.clone()); }
					"main.abandon_game" => {
						eprintln!("* Deleting savegame at {} and shutting down...", self.savegame_filename.clone()); // DEBUG: announce game abandon
						let _ = self.delete_game(self.savegame_filename.clone()); // WARN: may want to trap this error?
						self.set_mode(EngineMode::Offline);
					}
					"main.quit"      => {
						eprintln!("* Engine is shutting down..."); // DEBUG: announce engine shutdown
						self.set_mode(EngineMode::Offline);
					}
					_ => {
						eprintln!("! unhandled option '{}' selected from menu", item); // DEBUG: announce unhandled option
					}
				}
			}
		}
		for events in self.menu_context.drain_events() {
			match events {
				MenuEvent::Selected(event) => {
					eprintln!("* {:?}", event); // DEBUG: announce the context event that got matched
					if event.is_valid() {
						eprintln!("* Dispatching event..."); // DEBUG: announce event dispatch
						let event_handler = &mut self.bevy.world.get_resource_mut::<Events<GameEvent>>().unwrap();
						event_handler.send(event);
					}
					// WARN: In theory this should be the only GameEventType that comes through here, no guarantees though!
					if let GameEventType::PlayerAction(action) = event.etype {
						match action {
							ActionType::NoAction => { }
							ActionType::Examine => {
								eprintln!("* tried to Examine"); // DEBUG: report a detected EXAMINE event
							}
							_ => { }
						}
					}
				}
			}
		}
		// Execute variant behavior based on the engine's current EngineMode
		match self.mode {
			EngineMode::Offline => {
				eprintln!("* ! tick() called while mode == Offline, will now quit()"); // DEBUG: announce engine shutdown
				self.quit();
			}
			EngineMode::Standby => { // Any Engine state where normal operations have been temporarily suspended
				/* nothing to do in this mode for now */
			}
			EngineMode::Startup => {
				// the pre-/post-game context, when the game is not loaded but the main menu shows
				// Setup is all done, proceed with the game
				eprintln!("* Startup is complete"); // DEBUG: announce engine startup
				self.set_mode(EngineMode::Running);
			}
			EngineMode::Running => {
				/* the main running mode of the game */
				self.bevy.update();
			}
			EngineMode::Paused  => {
				/* halts the execution/processing of the game state vs Running */
			}
			EngineMode::GoodEnd => {
				/* VICTOLY */
			}
			EngineMode::BadEnd  => {
				/* DEFEAT  */
			}
		}
	}
	/// Master render method, invoking this will redraw the entire screen
	pub fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		// If the layout is dirty, recalculate it
		if self.layout_changed { self.solve_layout(frame.size()); }
		let default_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::White).bg(Color::Black));
		// If the engine is in standby mode, defer immediately
		if self.standby { self.render_main_menu(frame); return; }
		// If there's a valid CameraView to render, use that
		let p_posn = *self.bevy.world.get_resource::<Position>().unwrap();
		if let Some(mut view) = self.bevy.world.get_resource_mut::<CameraView>() {
			if self.visible_menu == MenuType::Context {
				if let Some(target) = self.menu_context.target {
					if target != Position::INVALID {
						view.reticle = target.to_camera_coords(self.ui_grid.camera_main, p_posn);
					}
				}
			} else if view.reticle != Position::INVALID {
				view.reticle = Position::INVALID;
			}
			frame.render_widget(Viewport::new(&view).block(default_block), self.ui_grid.camera_main);
		} else {
			frame.render_widget(Block::default().title("[no CameraView initialized]"), self.ui_grid.camera_main);
		}
		if self.visible_menu != MenuType::None {
			match self.visible_menu {
				MenuType::Main   => { self.render_main_menu(frame); }
				MenuType::Context => { self.render_context_menu(frame); }
				_ => { }
			}
		}
		self.render_planq(frame); // PLANQ is smart and will change appearance based on its state relative to the player
		self.render_message_log(frame); // Always render this
		// Display the fancy "PAUSED" banner if the game is paused
		if self.mode == EngineMode::Paused {
			let xpfile = &XpFile::from_resource("../resources/big_pause.xp").unwrap();
			let graphic = load_rex_pgraph(xpfile);
			let banner_area = Rect::new(10, 5, graphic.width() as u16, (graphic.height() + 2) as u16);
			let banner_img = Paragraph::new(graphic).block(Block::default().borders(Borders::TOP | Borders::BOTTOM));
			frame.render_widget(Clear, banner_area);
			frame.render_widget(banner_img, banner_area);
		} else if self.mode == EngineMode::GoodEnd {
			eprintln!("*************************");
			eprintln!("*** Victory detected! ***");
			eprintln!("*************************");
			self.quit();
		}
	}
	/// Renders the main menu, using the main menu object
	pub fn render_main_menu<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		//eprintln!("*** rendering main menu"); // DEBUG: announce main menu render event
		let menu = Menu::new().block(Block::default()
			                           .borders(Borders::TOP | Borders::RIGHT)
			                           .border_style(Style::default().fg(Color::White).bg(Color::DarkGray))
			                           .title("MAIN".to_string()));
		let area = Rect::new(self.menu_posn.0, self.menu_posn.1, self.menu_main.width as u16, 1);
		frame.render_stateful_widget(menu, area, &mut self.menu_main);
	}
	/// Renders the context menu, using the common context menu object
	pub fn render_context_menu<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		let menu = Menu::new().block(Block::default()
			                           .borders(Borders::TOP | Borders::RIGHT)
			                           .border_style(Style::default().fg(Color::White).bg(Color::DarkGray))
			                           .title("CONTEXT".to_string()));
		let area = Rect::new(self.menu_posn.0, self.menu_posn.1, self.menu_context.width as u16, 1);
		frame.render_stateful_widget(menu, area, &mut self.menu_context)
	}
	/// Renders the PLANQ sidebar object
	pub fn render_planq<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		{ // This has to be wrapped in a sub-scope to keep the borrow checker happy
			let monitor = self.bevy.world.get_resource::<PlanqMonitor>().unwrap();
			self.ui_grid.p_status_height = monitor.status_bars.len();
		}
		let mut planq = self.bevy.world.get_resource_mut::<PlanqData>().unwrap();
		// TODO: optimize this to only fire if the number of status bars actually changes
		self.ui_grid.calc_planq_layout(self.ui_grid.planq_sidebar);
		// Display some kind of 'planq offline' state if not carried
		// TODO: replace the 'no planq detected' message with something nicer
		if !planq.is_carried { // Player is not carrying a planq
			frame.render_widget(
				Paragraph::new("[no PLANQ detected]").block(
					Block::default().borders(Borders::NONE)
				),
				self.ui_grid.planq_status,
			);
			return;
		}
		// Display the terminal window if it's been set to visible
		if planq.show_terminal {
			planq.render_terminal(frame, self.ui_grid.planq_stdout);
			// Only display the CLI if there's a terminal visible to contain it
			if planq.show_cli_input {
				planq.render_cli(frame, self.ui_grid.planq_stdin, &mut self.planq_stdin);
			}
		}
		// Always render the status widgets: need to provide battery power, ship time, PLANQ status
		// WARN: this MUST be after we are done with the planq object above due to borrow checking
		let mut monitor = self.bevy.world.get_resource_mut::<PlanqMonitor>().unwrap();
		monitor.render(frame, self.ui_grid.planq_status);
	}
	/// Renders the message log pane at the bottom
	pub fn render_message_log<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		// Obtain a slice of the message log here and feed to the next widget
		let msglog_ref = self.bevy.world.get_resource::<MessageLog>();
		let msglog = msglog_ref.unwrap_or_default(); // get a handle on the msglog service
		if msglog_ref.is_some() {
			let worldmsg = msglog.get_log_as_lines("world".to_string(), 0); // get the full backlog
			/* WARN: magic number offset for window borders
			 * NOTE: it would be possible to 'reserve' space here by setting the magic num offset
			 *       greater than is strictly required to cause scrollback
			 */
			// Strict attention to typing required here lest we cause subtraction overflow errs
			let backlog_start_offset = (worldmsg.len() as i32) - self.ui_grid.msg_world.height as i32 + 2;
			let mut backlog_start: usize = 0;
			if backlog_start_offset > 0 { backlog_start = backlog_start_offset as usize; }
			let backlog = worldmsg[backlog_start..].to_vec(); // get a slice of the latest msgs
			// Draw the message log pane
			frame.render_widget(
				Paragraph::new(backlog).block( // requires a Vec<Spans<'a>> for group insert on creation
					Block::default()
					.borders(Borders::ALL)
					.border_style(Style::default().fg(Color::White))
				),
				self.ui_grid.msg_world,
			);
		}
	}
	/// Enables and places the given menu type at the specified position; should only need to be called at menu creation
	/// If the type is Main, then the menu does not need to be pre-populated
	pub fn set_menu(&mut self, m_type: MenuType, posn: (u16, u16)) {
		//eprintln!("* Enabling menu {:?} at {}, {}", m_type, posn.0, posn.1); // DEBUG: announce menu display
		if m_type == MenuType::Main {
			/*
			self.menu_main = MenuState::new(vec![
				MenuItem::item("New Game", "main.new_game".into()),
				MenuItem::item("Load Game", "main.load_game".into()),
				MenuItem::item("Save Game", "main.save_game".into()),
				MenuItem::item("Abandon", "main.abandon_game".into()),
				MenuItem::item("Quit", "main.quit".into()),
			]);
			*/
			let mut menu_items: Vec<MenuItem<Cow<'_, str>>> = Vec::new();
			menu_items.push(MenuItem::item("New Game", "main.new_game".into(), None));
			let filepath = bevy_save::get_save_file(&self.savegame_filename);
			if !self.standby {
				menu_items.push(MenuItem::item("Save Game", "main.save_game".into(), None));
			}
			if std::fs::metadata(filepath).is_ok() {
				menu_items.push(MenuItem::item("Load Game", "main.load_game".into(), None));
			}
			if !self.standby {
				menu_items.push(MenuItem::item("Abandon Game", "main.abandon_game".into(), None));
			}
			menu_items.push(MenuItem::item("Quit", "main.quit".into(), None));
			self.menu_main = MenuState::new(menu_items);
		}
		self.menu_posn = posn;
		self.visible_menu = m_type;
	}
	/// Helper for changing the current mode of the GameEngine
	pub fn set_mode(&mut self, new_mode: EngineMode) {
		eprintln!("* eng.mode set to {new_mode:?}"); // DEBUG: announce engine mode switch
		self.mode = new_mode;
	}
	/// Causes the GameEngine to halt and quit
	pub fn quit(&mut self) {
		// NOTE: this should probably instead execute a mode shift on the engine to allow for more graceful shutdowns
		self.running = false;
	}
	/// Starts a new game from scratch
	pub fn new_game(&mut self) {
		//eprintln!("* new_game() called"); // DEBUG: announce new game
		// If no game is running, then self.standby should be TRUE
		if !self.standby {
			eprintln!("* ! game is in progress!"); // DEBUG: warn about running game
			self.halt_game();
			self.standby = true;
			self.running = false;
		}
		self.init_bevy();
		self.build_new_worldmap();
		self.bevy.update();
		self.standby = false;
		self.running = true;
		self.set_mode(EngineMode::Running);
	}
	/// Stops and unloads a game-in-progress, ie before loading a new game or restarting
	pub fn halt_game(&mut self) {
		self.standby = true;
		self.set_mode(EngineMode::Standby);
		self.bevy = App::new();
		self.bevy.add_plugins(MinimalPlugins).add_plugins(SavePlugins);
	}
	/// Saves the currently-running game to an external file
	//  INFO: By default (not sure how to change this!), on Linux, this savegame will be at
	//      ~/.local/share/spacegame/saves/FILENAME.sav
	pub fn save_game(&mut self, filename: String) {
		// TODO: add an "are you sure" prompt
		eprintln!("* save_game() called on {}", filename); // DEBUG: alert when save_game is called
		if let Err(e) = self.bevy.world.save(&filename) {
			eprintln!("! ! save_game() failed on '{}', error: {}", filename, e); // DEBUG: warn about save game error
			return;
		}
		self.quit();
	}
	/// Loads a saved game from the given external file
	pub fn load_game(&mut self, filename: String) {
		// TODO: add an "are you sure" prompt
		eprintln!("* load_game() called on {} ({})", filename, self.standby); // DEBUG: alert when load_game is called
		if !self.standby {
			eprintln!("* ! game is in progress!"); // DEBUG: warn about running game
			self.halt_game();
			self.standby = true;
			self.running = false;
		}
		self.init_bevy();
		match self.bevy.world.load_applier(&filename) {
			Ok(applier) => {
				if let Err(f) = applier.despawn(DespawnMode::Unmapped).apply() {
					eprintln!( "! ERR: load_game() failed to apply the EntityMap, error: {}", f); // DEBUG: warn about loading error
				}
			}
			Err(e) => {
				eprintln!("! ERR: load_game() failed on '{}', error: {}", filename, e); // DEBUG: warn about loading error
			}
		}
		self.bevy.update();
		self.standby = false;
		self.running = true;
		self.set_mode(EngineMode::Running);
		eprintln!("* load_game() finished successfully"); // DEBUG: alert when load_game finishes
	}
	/// Deletes the game save, ie after dying or abandoning the game
	pub fn delete_game(&mut self, filename: String) -> std::io::Result<()> {
		eprintln!("* delete_game() called on {}", filename); // DEBUG: alert when delete_game is called
		let filepath = bevy_save::get_save_file(&filename);
		std::fs::remove_file(filepath)
	}
	/// Puts the game into a PAUSED state
	pub fn pause_game(&mut self) {
		self.set_mode(EngineMode::Paused);
	}
	/// Puts the game back into a RUNNING state
	pub fn unpause_game(&mut self) {
		self.set_mode(EngineMode::Running);
	}
	/// Toggles the game from paused to unpaused or vice versa
	pub fn pause_toggle(&mut self) {
		if self.mode == EngineMode::Paused {
			self.unpause_game();
		} else {
			self.pause_game();
		}
	}
	/// Gets Bevy instance set up from nothing, up to just before calling bevy.world.update()
	pub fn init_bevy(&mut self) {
		eprintln!("* Initializing Bevy..."); // DEBUG: announce Bevy startup
		let chanlist = vec!["world".to_string(),
			                  "planq".to_string(),
			                  "debug".to_string()];
		self.bevy
		//.add_event::<crossterm::event::KeyEvent>() // Registers the KeyEvent from crossterm in Bevy
		.add_plugins(RngPlugin::default())
		//.add_systems(Startup, new_player_spawn)
		.add_systems(Startup, (new_player_spawn,
			                     new_lmr_spawn,
		))
		.add_systems(Update, (action_referee_system,
			                    camera_update_system,
			                    examination_system,
			                    item_collection_system,
			                    lockable_system,
			                    map_indexing_system,
			                    movement_system,
			                    openable_system,
			                    operable_system,
			                    planq_update_system,
			                    planq_monitor_system,
			                    visibility_system,
		))
		//.register_saveable::<EngineMode>()
		//.register_saveable::<CameraView>()
		.register_saveable::<TileType>()
		.register_saveable::<Tile>()
		.register_saveable::<Map>()
		.register_saveable::<Model>()
		.register_saveable::<GameEventContext>()
		.register_saveable::<GameEventType>()
		.register_saveable::<GameEvent>()
		.register_saveable::<Message>()
		.register_saveable::<MessageChannel>()
		.register_saveable::<MessageLog>()
		//.register_saveable::<ItemBuilder>()
		.register_type::<Vec<String>>()
		.register_type::<Vec<Message>>()
		.register_type::<Vec<MessageChannel>>()
		.register_type::<Vec<TileType>>()
		.register_type::<Vec<Tile>>()
		.register_type::<Vec<Map>>()
		.register_type::<Vec<bool>>()
		.register_type::<(i32, i32, i32)>()
		.register_type::<Position>()
		.register_type::<HashMap<(i32, i32, i32), (i32, i32, i32)>>()
		.register_type::<HashMap<Entity, Position>>()
		.register_type::<bevy::utils::HashSet<ActionType>>()
		// from components.rs:
		.register_saveable::<Player>()
		.register_saveable::<ActionSet>()
		.register_saveable::<bevy::utils::hashbrown::HashSet<ActionType>>()
		.register_saveable::<Position>()
		.register_saveable::<Description>()
		.register_saveable::<Renderable>()
		.register_saveable::<Mobile>()
		.register_saveable::<Obstructive>()
		.register_saveable::<Portable>()
		.register_saveable::<Container>()
		.register_saveable::<Opaque>()
		.register_saveable::<Openable>()
		.register_saveable::<Planq>()
		.register_saveable::<Lockable>()
		.insert_resource(Events::<GameEvent>::default())
		.insert_resource(Events::<PlanqEvent>::default())
		.insert_resource(MessageLog::new(chanlist))
		.insert_resource(PlanqData::new())
		.insert_resource(PlanqMonitor::new())
		.insert_resource(Position::new(9, 9, 1)) // DEBUG: arbitrary player spawnpoint
		.insert_resource(RexAssets::new())
		;
		self.mode = EngineMode::Startup;
		self.solve_layout(self.term_dims);
		self.build_camera();
	}
	/// Creates the initial worldmap from scratch
	pub fn build_new_worldmap(&mut self) {
		let mut model = Model::default();
		self.mason = get_builder(1);
		self.mason.build_map();
		let mut worldmap = self.mason.get_map();
		// Construct the various furniture/scenery/backdrop items
		let item_spawns = self.mason.get_item_spawn_list();
		eprintln!("* item_spawns.len(): {}", item_spawns.len()); // DEBUG: announce number of spawning items
		for item in item_spawns.iter() {
			self.artisan.create(item.0).at(item.1).build(&mut self.bevy.world);
		}
		model.levels.push(worldmap);
		// Create a dev map and drop a portal to it
		self.mason = get_builder(68);
		self.mason.build_map();
		worldmap = self.mason.get_map();
		model.levels.push(worldmap);
		model.add_portal((3, 20, 0), (5, 5, 1), true);
		// Add the game world to the engine
		self.bevy.insert_resource(model);
	}
	/// Handles the initial spawns for a new game, esp those items that are not included with the worldmap layout
	pub fn populate_new_worldmap(&mut self) {
		todo!();
	}
	/// Creates a fallback dev map for testing purposes
	pub fn build_dev_worldmap(&mut self) {
		let mut model = Model::default();
		// Build the DevMapBasement
		self.mason.build_map();
		let mut worldmap = self.mason.get_map();
		// get_item_spawn_list();
		// artisan.spawn_batch(item_spawn_list);
		//self.artisan.spawn_at(&mut self.bevy.world, ItemType::Door, (10, 10, 0).into());
		self.artisan.create(ItemType::Door).at(Position::new(10, 10, 0)).build(&mut self.bevy.world);
		model.levels.push(worldmap);
		// Build the DevMapLobby
		self.mason = get_builder(2);
		self.mason.build_map();
		worldmap = self.mason.get_map();
		// get_item_spawn_list();
		// artisan.spawn_batch(item_spawn_list);
		//self.artisan.spawn_at(&mut self.bevy.world, ItemType::Door, (13, 17, 1).into());
		self.artisan.create(ItemType::Door).at(Position::new(13, 17, 1)).build(&mut self.bevy.world);
		model.levels.push(worldmap);
		// Add level transitions and teleporters
		model.add_portal((5, 5, 0), (7, 7, 1), true);
		// Finally, add the maps to the world model
		self.bevy.insert_resource(model);
	}
	/// Creates a new CameraView object with visibility onto the world map
	pub fn build_camera(&mut self) {
		// need to calculate the layout PRIOR to this point
		let main_camera = CameraView::new(self.ui_grid.camera_main.width as i32, self.ui_grid.camera_main.height as i32);
		self.bevy.insert_resource(main_camera);
	}
	/// Solves the layout configuration given a set of layout constraints and an area to cover
	pub fn solve_layout(&mut self, area: Rect) {
		self.ui_grid.calc_layout(area);
		if let Some(mut camera) = self.bevy.world.get_resource_mut::<CameraView>() {
			camera.set_dims(self.ui_grid.camera_main.width as i32, self.ui_grid.camera_main.height as i32);
		}
	}
	/// Requests the creation of an item from the item builder
	pub fn make_item(&mut self, new_type: ItemType, location: Position) {
		self.artisan.create(new_type).at(location).build(&mut self.bevy.world);
	}
	/// Requests to give a new Item to a specific Entity
	pub fn give_item(&mut self, new_type: ItemType, target: Entity) {
		self.artisan.create(new_type).within(target).build(&mut self.bevy.world);
	}
	/// Executes a command on the PLANQ, generally from the CLI
	pub fn exec(&mut self, cmd: PlanqCmd) -> bool {
		let mut msglog = self.bevy.world.get_resource_mut::<MessageLog>().unwrap();
		match cmd {
			PlanqCmd::Error(msg) => {
				msglog.tell_planq("¶│ ERROR:".to_string());
				msglog.tell_planq(format!("¶│ {}", msg));
				msglog.tell_planq(" ".to_string());
			}
			PlanqCmd::Help => {
				msglog.tell_planq("¶│ Available commands:".to_string());
				for command in PlanqCmd::iter() {
					msglog.tell_planq(format!("¶│   {}", command));
				}
				msglog.tell_planq(" ".to_string());
			}
			PlanqCmd::Shutdown => { todo!(); /* trigger a shutdown */ }
			PlanqCmd::Reboot => { todo!(); /* execute a reboot */ }
			PlanqCmd::Connect(target) => { todo!(); /* run the planq.connect subroutine */ }
			PlanqCmd::Disconnect => { todo!(); /* run the planq.disconnect subroutine */ }
			_ => { /* NoOperation */ }
		}
		false
	}
}

// *** TYPES, HELPERS, and SINGLETONS
/// Application result type, provides some nice handling if the game crashes
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Defines the set of modes that the GameEngine may run in during the course of the program
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Resource)]
pub enum EngineMode {
	#[default]
	Offline,
	Standby,    // ie when showing the startup menu, victory/game over screens, &c
	Startup,
	Running,
	Paused,
	GoodEnd,
	BadEnd,     // TODO: set up variants for both this and GoodEnd? maybe just a GameOver mode?
}

/// Provides a bunch of named fields (rather than a tuple) of grid components
/// # Fields
/// * `camera_main`     Contains the player's view of the meatspace game world
/// * `msg_world`       Contains the world-level message backlog
/// * `planq_sidebar`   The *entire* PLANQ area, including borders, without subdivisions
/// * `planq_status`    The PLANQ's status bars, at the top
/// * `planq_screen`    The PLANQ's entire terminal view, dynamically sized to leave room for status bars
/// * `planq_stdout`    The part of the _screen that contains the terminal's backscroll
/// * `planq_stdin`     The PLANQ's CLI input box
/// * 'p_status_height' Sets the height of the status bar widget
/// * 'p_stdin_height'  Sets the height of the CLI input widget
pub struct UIGrid {
	/// Provides the main view onto the worldmap
	pub camera_main:      Rect,
	/// Designates the 'default' message log, which always shows msgs from the World channel
	pub msg_world:        Rect,
	/// Designates the area for the whole Planq sidebar, all panels included
	pub planq_sidebar:    Rect,
	/// Designates the space reserved for the Planq's stats: offline status, battery power, &c
	pub planq_status:     Rect,
	/// Designates the space for the Planq's entire terminal
	pub planq_screen:     Rect,
	/// Designates the output screen of the Planq
	pub planq_stdout:     Rect,
	/// Designates the CLI input of the Planq
	pub planq_stdin:      Rect,
	/// Sets the height of the planq_status widget, will be updated during gameplay
	pub p_status_height:  usize,
	/// Sets the height of the planq's CLI widget
	pub p_stdin_height:   usize
}
impl UIGrid {
	pub fn new() -> UIGrid {
		UIGrid {
			camera_main: Rect::default(),
			msg_world: Rect::default(),
			planq_sidebar: Rect::default(),
			planq_status: Rect::default(),
			planq_screen: Rect::default(),
			planq_stdout: Rect::default(),
			planq_stdin: Rect::default(),
			p_status_height: 0,
			p_stdin_height: 1,
		}
	}
	/// Recalculates the PLANQ's layout based on its stored size
	/// Should take into account the dynamic modules, prevent overlap,
	/// and writes its results to the planq_status, planq_screen,
	/// planq_stdout, and planq_stdin fields of the UIGrid object.
	pub fn calc_planq_layout(&mut self, max_area: Rect) {
		// NEW METHOD for PLANQ splits
		// (as a method call somewhere else, so that it can be redone outside of here)
		// given the full width W and height H of the render area,
		// 1- obtain the height of the planq_status module(s), H
		//    (this can be 0 but should be more as the planq_status has some builtins)
		// 2- split H between Max(I) and Min(4) into planq_status and planq_screen,
		//    so that the CLI's stdout will flow to fill the leftover space
		// 3- split planq_screen along the vertical as Min(1), Max(J) where J is the height
		//    of the PLANQ's stdin module, probably = 1 (but not guaranteed!)
		// 4- store these splits on the UI grid:
		//    planq - W, H
		//    \_planq_status - I
		//    \_planq_screen
		//      \_planq_stdout
		//      \_planq_stdin - J
		// ---
		// max_area provides the entire space allowed to this widget
		let first_split = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(self.p_status_height as u16), Constraint::Min(4)].as_ref())
			.split(max_area).to_vec();
		let second_split = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(1), Constraint::Max(self.p_stdin_height as u16)].as_ref())
			.split(first_split[1]).to_vec();
		self.planq_status = first_split[0];
		self.planq_screen = first_split[1];
		self.planq_stdout = second_split[0];
		self.planq_stdin = second_split[1];
	}
	/// Recalculates the UI layout based on the given size, to be invoked if the screen is resized
	pub fn calc_layout(&mut self, max_area: Rect) {
		/* Use the layout to build up the UI and its contents
		 * - iterate through the layout stack
		 * - if the object indexed to the layout Rect is active, then draw it
		 * frame.render_widget(self, Widget, area: Rect)
		 * - might consider nesting the calls:
		 *   draw_thing<Backend>(f: &mut Frame<Backend>, app: &mut App, area: Rect)
		 * TODO: one day i'll have the time to make this dynamic/rearrangable...
		 * MAIN LAYOUT
		 * +----+-+
		 * | 1  | |
		 * |    |3|
		 * +----+ |
		 * | 2  | |
		 * +----+-+
		 * block 1 is the overworld camera
		 *  - dims: min: w30, h30, max: fill
		 * block 2 is the PLANQ output and message log
		 *  - dims: min: w(B1), h5+1, max: fill
		 * block 3 is the status output stack
		 *  - layout within block 3 is handled by its internal logic
		 *  - dims: min: w10, h(S), max: w20, h(S)
		 * Cogmind uses a minimum 'grid' size of 80 wide by 60 high, seems legit
		 */
		// Recalculate everything given the new area
		// Split the entire window between [1/2](0) and [3](1) horizontally
		let main_horiz_split = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Min(30), Constraint::Length(38)].as_ref())
			.split(max_area).to_vec();
		// Split [1](0) and [2](1) vertically
		let camera_worldmsg_split = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(30), Constraint::Length(12)].as_ref())
			.split(main_horiz_split[0]).to_vec();
		// Update the UIGrid itself to hold the new sizes
		self.camera_main = camera_worldmsg_split[0];
		self.msg_world = camera_worldmsg_split[1];
		self.planq_sidebar = main_horiz_split[1];
		self.calc_planq_layout(self.planq_sidebar);
	}
}
impl Default for UIGrid {
	fn default() -> UIGrid {
		UIGrid::new()
	}
}


// EOF
