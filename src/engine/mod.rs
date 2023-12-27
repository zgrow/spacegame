// engine/mod.rs
// July 12 2023

// ###: EXTERNAL LIBS
use std::borrow::Cow;
use std::error;
use std::path::{Path, PathBuf};
use bevy::{
	prelude::*,
	utils::{HashMap, HashSet},
};
use bevy_turborand::prelude::*;
use bracket_rex::prelude::*;
use moonshine_save::prelude::*;
use ratatui::{
	prelude::*,
	Frame,
	backend::Backend,
	layout::Rect,
	style::{
		Color,
		Style
	},
	widgets::*,
};
use strum::IntoEnumIterator;

// ###: INTERNAL LIBS
pub mod event;
pub mod handler;
pub mod menu;
pub mod messagelog;
pub mod tui;
pub mod viewport;
use crate::{
	artisan::*,
	camera::*,
	components::*,
	engine::{
		event::*,
		menu::*,
		messagelog::*,
		tui::*,
		viewport::Viewport,
	},
	mason::{
		get_world_builder,
		logical_map::*,
		rexpaint_loader::load_rex_pgraph,
		WorldBuilder,
	},
	planq::*,
	planq::monitor::*,
	planq::tui::*,
	rex_assets::*,
	sys::*,
	worldmap::*,
};

// ###: COMPLEX TYPES
//  ##: GameEngine
pub struct GameEngine<'a> {
	pub running:        bool, // If true, the game loop is running
	pub standby:        bool, // If true, the game loop is on standby (ie paused)
	pub mode:           EngineMode,
	pub bevy:           App, // bevy::app::App, contains all of the ECS and related things
	pub mason:          Box<dyn WorldBuilder>,
	pub artisan:        ItemBuilder,
	pub visible_menu:   MenuType,
	pub menu_main:      MenuState<Cow<'static, str>>,
	pub menu_context:   MenuState<GameEvent>,
	pub menu_posn:      (u16, u16),
	pub ui_grid:        UIGrid,
	pub layout_changed: bool,
	pub default_block:  Block<'a>,
	pub default_style:  Style,
	pub savegame_filename: String,
	pub term_dims:      Rect,
	pub planq_stdin:    PlanqInput<'a>,
}
impl GameEngine<'_> {
	/// Constructs a new instance of [`GameEngine`].
	pub fn new(max_area: Rect) -> Self {
		let mut new_eng = GameEngine {
			running: false,
			standby: true,
			mode: EngineMode::Standby,
			bevy: App::new(),
			mason: get_world_builder(),
			artisan: ItemBuilder::new(),
			// HINT: These menu items are handled via a match case in GameEngine::tick()
			visible_menu: MenuType::None,
			menu_main: MenuState::new(vec![]),
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
		new_eng.planq_stdin.input.set_cursor_line_style(Style::default().fg(Color::Yellow).bg(Color::Black));
		new_eng.bevy.add_plugins(MinimalPlugins).add_plugins((SavePlugin, LoadPlugin));
		new_eng
	}
	/// Runs a single update cycle of the GameEngine
	pub fn tick(&mut self) {
	/* HINT: This is a known-good local method for obtaining data from a selected entity
	 *	_ => {
	 *		error!("! unhandled option '{}' selected from menu", item); // DEBUG: report an unhandled menu option
	 *		let enty_id = item.parse::<u32>().unwrap();
	 *		let enty_ref = self.bevy.world.entities().resolve_from_id(enty_id);
	 *		if let Some(enty) = enty_ref {
	 *			if self.bevy.world.entities().contains(enty) {
	 *				debug!("* produced a valid enty_ref from an entity.index()"); // DEBUG: report entity reference success
	 *			if let Some(name) = self.bevy.world.get::<ActorName>(enty) {
	 *					debug!("* Entity {} named {} was selected", enty_id, name.name.clone()); // DEBUG: announce entity selection
	 *				} else {
	 *					warn!("* Could not retrieve the name of the selected entity"); // DEBUG: report entity component retrieval failure
	 *				}
	 *			}
	 *		}
	 *	}
	 */
		// This is where I'd pull any mode changes that might have happened during the last Bevy update and apply them
		//if settings.mode_changed { ... }
		// If the game is supposed to shut down, then do so
		if let Some(quit_event) = self.bevy.world.get_resource::<Events<QuitRequest>>() {
			if !quit_event.is_empty() {
				debug!("* detected QuitRequest via tick()");
				self.set_mode(EngineMode::Offline);
			}
		}
		// If the game was just restarted, then invoke the startup sequence
		if let Some(start_event) = self.bevy.world.get_resource::<Events<StartRequest>>() {
			if !start_event.is_empty() {
				debug!("* detected StartRequest via tick()");
				self.bevy.update();
				self.standby = false;
				self.running = true;
				self.set_mode(EngineMode::Running);
			}
		}
		// If there are any menu events, handle them
		for event in self.menu_main.drain_events() {
			// NOTE: if the user selects a submenu heading as their choice, *nothing* will be generated; the menu will just close
			//       not sure yet if there's a way to trap that outcome
			match event {
				MenuEvent::Selected(item) => match item.as_ref() {
					"main.new_game"  => { self.new_game(); }
					"main.load_game" => {
						debug!("* Now sending LoadRequest");
						self.bevy.world.send_event(LoadRequest{ path: self.savegame_filename.clone().into() });
					}
					"main.save_game" => {
						debug!("* Now sending SaveRequest");
						self.bevy.world.send_event(SaveRequest{ path: self.savegame_filename.clone().into() });
					}
					"main.abandon_game" => {
						info!("* Deleting savegame at {} and shutting down...", self.savegame_filename.clone()); // DEBUG: announce game abandon
						let _ = self.delete_game(&self.savegame_filename.clone()); // WARN: may want to trap this error?
						self.set_mode(EngineMode::Offline);
					}
					"main.quit"      => {
						info!("* Engine is shutting down..."); // DEBUG: announce engine shutdown
						self.set_mode(EngineMode::Offline);
					}
					_ => {
						error!("! unhandled option '{}' selected from menu", item); // DEBUG: announce unhandled option
					}
				}
			}
		}
		for events in self.menu_context.drain_events() {
			match events {
				MenuEvent::Selected(event) => {
					trace!("* tick(): menu event: {:?}", event); // DEBUG: announce the context event that got matched
					if event.is_valid() {
						if let Some(event_handler) = &mut self.bevy.world.get_resource_mut::<Events<GameEvent>>() {
							event_handler.send(event);
						}
					}
					// WARN: In theory this should be the only GameEventType that comes through here, no guarantees though!
					if let GameEventType::PlayerAction(action) = event.etype {
						match action {
							ActionType::NoAction => { }
							ActionType::Examine => {
								//debug!("* tried to Examine"); // DEBUG: report a detected EXAMINE event
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
				warn!("* tick() called while mode == Offline, will now quit()"); // DEBUG: announce engine shutdown
				self.quit();
			}
			EngineMode::Standby => { // Any Engine state where normal operations have been temporarily suspended
				/* nothing to do in this mode for now */
			}
			EngineMode::Startup => {
				// the pre-/post-game context, when the game is not loaded but the main menu shows
				// Setup is all done, proceed with the game
				//debug!("* Startup is complete"); // DEBUG: announce engine startup
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
		// Try to get the player's position out of Bevy
		let p_posn: Position = *self.bevy.world.get_resource::<Position>().unwrap_or(&Position::INVALID);
		// If there's a valid CameraView to render, use that
		if let Some(mut view) = self.bevy.world.get_resource_mut::<CameraView>() {
			if self.visible_menu == MenuType::Context {
				if let Some(target) = self.menu_context.target {
					if target != Position::INVALID && p_posn.is_valid() {
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
		// If there's a visible menu, render that too
		if self.visible_menu != MenuType::None {
			match self.visible_menu {
				MenuType::Main   => { self.render_main_menu(frame); }
				MenuType::Context => { self.render_context_menu(frame); }
				_ => { }
			}
		}
		// PLANQ is smart and will change appearance based on its state relative to the player
		self.render_planq(frame);
		// Always render the message log
		self.render_message_log(frame);
		// Display the fancy "PAUSED" banner if the game is paused
		if self.mode == EngineMode::Paused {
			if let Ok(xpfile) = &XpFile::from_resource("../resources/big_pause.xp") {
				let graphic = load_rex_pgraph(xpfile);
				let banner_area = Rect::new(10, 5, graphic.width() as u16, (graphic.height() + 2) as u16);
				let banner_img = Paragraph::new(graphic).block(Block::default().borders(Borders::TOP | Borders::BOTTOM));
				frame.render_widget(Clear, banner_area);
				frame.render_widget(banner_img, banner_area);
			}
		} else if self.mode == EngineMode::GoodEnd {
			info!("*************************");
			info!("*** Victory detected! ***");
			info!("*************************");
			self.quit();
		}
	}
	/// Renders the main menu, using the main menu object
	pub fn render_main_menu<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		//debug!("* rendering main menu"); // DEBUG: announce main menu render event
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
		if let Some(monitor) = self.bevy.world.get_resource::<PlanqMonitor>() {
			self.ui_grid.p_status_height = monitor.status_bars.len();
		}
		if let Some(mut planq) = self.bevy.world.get_resource_mut::<PlanqData>() {
			self.ui_grid.calc_planq_layout(self.ui_grid.planq_sidebar);
			// Display some kind of 'planq offline' state if not carried
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
		}
		// Always render the status widgets: need to provide battery power, ship time, PLANQ status
		// WARN: this MUST be after we are done with the planq object above due to borrow checking
		if let Some(mut monitor) = self.bevy.world.get_resource_mut::<PlanqMonitor>() {
			monitor.render(frame, self.ui_grid.planq_status);
		}
	}
	/// Renders the message log pane at the bottom
	pub fn render_message_log<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		// Obtain a slice of the message log here and feed to the next widget
		let msglog_ref = self.bevy.world.get_resource::<MessageLog>();
		let msglog = msglog_ref.unwrap_or_default(); // get a handle on the msglog service
		if msglog_ref.is_some() {
			let worldmsg = msglog.get_log_as_lines("world", 0); // get the full backlog
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
				Paragraph::new(Text::from(backlog)) // requires a Vec<Line<'a>> for group insert on creation
				.block(
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
		//debug!("* Enabling menu {:?} at {}, {}", m_type, posn.0, posn.1); // DEBUG: announce menu display
		if m_type == MenuType::Main {
			let mut menu_items: Vec<MenuItem<Cow<'_, str>>> = Vec::new();
			menu_items.push(MenuItem::item("New Game", "main.new_game".into(), None));
			if !self.standby {
				menu_items.push(MenuItem::item("Save Game", "main.save_game".into(), None));
			}
			//let filepath = bevy_save::get_save_file(&self.savegame_filename);
			if std::fs::metadata(Path::new(&self.savegame_filename)).is_ok() {
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
		//debug!("* eng.mode set to {new_mode:?}"); // DEBUG: announce engine mode switch
		self.mode = new_mode;
	}
	/// Causes the GameEngine to halt and quit
	pub fn quit(&mut self) {
		self.running = false;
	}
	/// Starts a new game from scratch
	pub fn new_game(&mut self) {
		// If no game is running, then self.standby should be TRUE
		if !self.standby {
			warn!("* ! game is in progress!"); // DEBUG: warn about running game
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
		self.bevy.add_plugins(MinimalPlugins).add_plugins((SavePlugin, LoadPlugin));
	}
	/// Deletes the game save, ie after dying or abandoning the game
	pub fn delete_game(&mut self, filename: &str) -> std::io::Result<()> {
		//debug!("* delete_game() called on {}", filename); // DEBUG: alert when delete_game is called
		//let filepath = bevy_save::get_save_file(filename);
		std::fs::remove_file(Path::new(filename))
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
		//debug!("* Initializing Bevy..."); // DEBUG: announce Bevy startup
		let chanlist = vec!["world".to_string(),
			                  "planq".to_string(),
			                  "debug".to_string()];
		self.bevy
		.add_plugins(RngPlugin::default()) // Non-deterministic RNG
		//.add_plugins(RngPlugin::new().with_rng_seed(69420)) // Forces the RNG to be deterministic
		.add_systems(PreUpdate, (raise_quit_event_after_saving_game.in_set(SaveSet::PostSave),
			                       raise_start_event_after_loading_game.in_set(LoadSet::PostLoad),
														 load_saved_game_system.in_set(LoadSet::PostLoad).after(load_from_file_on_event::<LoadRequest>),
			                       load_from_file_on_event::<LoadRequest>(),
			                       save_default()
			                         .include_resource::<CameraView>()
			                         .include_resource::<MessageLog>()
															 .include_resource::<PlanqData>()
															 .include_resource::<PlanqMonitor>()
			                         .include_resource::<Position>()
			                         .include_resource::<WorldModel>()
			                         .into_file_on_event::<SaveRequest>()
		))
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
		.add_event::<LoadRequest>()
		.add_event::<QuitRequest>()
		.add_event::<SaveRequest>()
		.add_event::<StartRequest>()
		.register_type::<AccessPort>()
		.register_type::<ActionSet>()
		.register_type::<ActionType>()
		.register_type::<Body>()
		.register_type::<CameraView>()
		.register_type::<Container>()
		.register_type::<DataSampleTimer>()
		.register_type::<Description>()
		.register_type::<Device>()
		.register_type::<DeviceState>()
		.register_type::<crate::components::Direction>()
		.register_type::<EngineMode>()
		.register_type::<GameEvent>()
		.register_type::<GameEventContext>()
		.register_type::<GameEventType>()
		.register_type::<GlobalRng>()
		.register_type::<Glyph>()
		.register_type::<GraphCell>()
		.register_type::<GraphDoor>()
		.register_type::<GraphRoom>()
		.register_type::<IsCarried>()
		.register_type::<ItemBuilder>()
		.register_type::<ItemRequest>()
		.register_type::<Key>()
		.register_type::<LMR>()
		.register_type::<Lockable>()
		.register_type::<Memory>()
		.register_type::<Message>()
		.register_type::<MessageChannel>()
		.register_type::<MessageLog>()
		.register_type::<Mobile>()
		.register_type::<Networkable>()
		.register_type::<Obstructive>()
		.register_type::<Opaque>()
		.register_type::<Openable>()
		.register_type::<Planq>()
		.register_type::<PlanqActionMode>()
		.register_type::<PlanqCPUMode>()
		.register_type::<PlanqData>()
		.register_type::<PlanqDataType>()
		.register_type::<PlanqEvent>()
		.register_type::<PlanqEventType>()
		.register_type::<PlanqMonitor>()
		.register_type::<PlanqProcess>()
		.register_type::<Player>()
		.register_type::<Portable>()
		.register_type::<Portal>()
		.register_type::<Position>()
		.register_type::<RngComponent>()
		.register_type::<ScreenCell>()
		.register_type::<ShipGraph>()
		.register_type::<Tile>()
		.register_type::<TileType>()
		.register_type::<TimerMode>()
		.register_type::<Viewshed>()
		.register_type::<WorldMap>()
		.register_type::<WorldModel>()
		.register_type::<(i32, Entity)>()
		.register_type::<(i32, i32, i32)>()
		.register_type::<Option<usize>>()
		.register_type::<HashSet<ActionType>>()
		.register_type::<HashMap<(i32, i32, i32), (i32, i32, i32)>>()
		.register_type::<HashMap<Entity, Position>>() // planned to be superceded by the below type
		.register_type::<HashMap<Position, Position>>()
		.register_type::<HashMap<Position, ScreenCell>>()
		.register_type::<HashMap<Position, Vec<Entity>>>()
		.register_type::<HashMap<String, PlanqDataType>>()
		.register_type::<Vec<bool>>()
		.register_type::<Vec<String>>()
		.register_type::<Vec<Entity>>()
		.register_type::<Vec<Glyph>>()
		.register_type::<Vec<GraphDoor>>()
		.register_type::<Vec<GraphRoom>>()
		.register_type::<Vec<Message>>()
		.register_type::<Vec<MessageChannel>>()
		.register_type::<Vec<Portal>>()
		.register_type::<Vec<Position>>()
		.register_type::<Vec<ScreenCell>>()
		.register_type::<Vec<Tile>>()
		.register_type::<Vec<TileType>>()
		.register_type::<Vec<WorldMap>>()
		.register_type::<Vec<(i32, Entity)>>()
		.insert_resource(Events::<GameEvent>::default())
		.insert_resource(Events::<PlanqEvent>::default())
		.insert_resource(MessageLog::new(chanlist))
		.insert_resource(PlanqData::new())
		.insert_resource(PlanqMonitor::new())
		.insert_resource(Position::new(4, 14, 1)) // DEBUG: arbitrary player spawnpoint
		.insert_resource(RexAssets::new())
		;
		self.mode = EngineMode::Startup;
		self.solve_layout(self.term_dims);
		self.build_camera();
	}
	/// Creates the initial worldmap from scratch
	pub fn build_new_worldmap(&mut self) {
		// Loads the generated JSON layout file and parses it out into the game's data structures:
		// - Creates the 'physical' tilemaps of ScreenCells that represent the game's terrain
		// - Creates the 'logical' topology map of GraphRooms/GraphPortals that provide pathfinding and placement
		// - Generates the baseline list of doors required to connect all of the rooms in the map
		// - Generates the list of 'ladders' that connect rooms across z-levels and allow movement
		let mut rng = self.bevy.world.get_resource_mut::<GlobalRng>().expect("RNG should always be an available Bevy resource");
		self.mason.build_world(); // <- remove the RNG from here for starters, insert it closer to where it's needed
		// Get a copy of the freshly-constructed world model
		let mut model = self.mason.get_model();
		let mut new_item_list = Vec::new();

		// Get the list of items that we know for sure need to be generated at specific positions
		let mut item_spawns = self.mason.get_essential_item_requests(); // list of (name, posn)
		//eprintln!("* DEBUG: build_new_worldmap: essential: {:?}", item_spawns);
		new_item_list.append(&mut item_spawns);
		// Next, get the list of requested items, find spawnpoints for them, and add them to the list of spawns
		let item_reqs = self.mason.get_additional_item_requests();
		//eprintln!("* DEBUG: build_new_worldmap: additional: {:?}", item_reqs); // DEBUG:
		for (room_name, item_name) in item_reqs.iter() {
			//eprintln!("* DEBUG: Attempting to spawn {} in {}", item_name, room_name); // DEBUG:
			// get the item shape from artisan (returns a SpawnTemplate)
			//eprintln!("** DEBUG: looking to get a shape for {}", item_name);
			if let Some(item_shape) = self.artisan.get_random_shape(item_name, &mut rng) {
				// try to get a spawnpoint from mason using the ItemTemplate (returns a Option<Vec<(name: String, ref_posn: Position)>>)
				//eprintln!("*** DEBUG: looking to get a spawnpoint for {}", item_name);
				if let Some(mut item_spawns) = model.find_spawnpoint_in(room_name, item_shape, &mut rng) {
					//eprintln!("**** DEBUG: found a place to spawn {}: {:?}", item_name, item_spawns);
					new_item_list.append(&mut item_spawns);
				}
			}
		}
		// Spawn all of the items we need for the game
		// This CANNOT be executed in the loop above or Rust will complain about a double borrow
		// WARN: Need to have *all* positions decided on by this point
		//eprintln!("* DEBUG: Sending the following list for spawn:\n{:#?}", new_item_list); // DEBUG:
		for (i_name, i_posn) in new_item_list.iter() {
			let item_list = self.artisan.create(i_name).at(*i_posn).build(&mut self.bevy.world);
			for (i_enty, i_shape) in item_list.iter() {
				model.add_contents(i_shape, 0, i_enty.id());
				//debug!("* added new item '{}' at posn {:?}", i_name, i_posn);
				//eprintln!("DEBUG: * added new item '{}' at posn {:?}", i_name, i_posn);
			}
		}
		// Add the fully-constructed world model to Bevy
		self.bevy.insert_resource(model);
	}
	/// DEBUG: Creates a fallback dev map for testing purposes
	pub fn build_dev_worldmap(&mut self) {
		/* disabled because i don't feel like updating it right now since the json loader works
		let mut model = Model::default();
		// Build the DevMapBasement
		self.mason.build_map();
		let mut worldmap = self.mason.get_map();
		//get_item_spawn_list();
		//artisan.spawn_batch(item_spawn_list);
		//self.artisan.spawn_at(&mut self.bevy.world, ItemType::Door, (10, 10, 0).into());
		self.artisan.create(ItemType::Door).at((10, 10, 0).into()).build(&mut self.bevy.world);
		model.levels.push(worldmap);
		// Build the DevMapLobby
		self.mason = get_map_builder(2);
		self.mason.build_map();
		worldmap = self.mason.get_map();
		//get_item_spawn_list();
		//artisan.spawn_batch(item_spawn_list);
		//self.artisan.spawn_at(&mut self.bevy.world, ItemType::Door, (13, 17, 1).into());
		self.artisan.create(ItemType::Door).at((13, 17, 1).into()).build(&mut self.bevy.world);
		model.levels.push(worldmap);
		// Add level transitions and teleporters
		model.add_portal((5, 5, 0).into(), (7, 7, 1).into(), true);
		// Finally, add the maps to the world model
		self.bevy.insert_resource(model);
		*/
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
	/// Executes a command on the PLANQ, generally from the CLI; DEBUG: always returns false
	pub fn exec(&mut self, cmd: PlanqCmd) -> bool {
		// FIXME: this unwrap() cannot be replaced in situ, because regardless of whether or not there's a MessageLog,
		// the PLANQ's commands should still be executed!
		// Therefore, it would be better to pull all of these msglog-unwrap-tell_planq chains out to their own
		// dedicated method, as self.tell_planq(), which itself handles these parts and can safely handle
		// the unwrapping logic
		let mut msglog = self.bevy.world.get_resource_mut::<MessageLog>().expect("MessageLog should be in Bevy");
		match cmd {
			PlanqCmd::Error(msg) => {
				msglog.tell_planq("[[fg:yellow]]¶[[fg:gray]]│[[fg:red]]ERROR:");
				msglog.tell_planq(format!("[[fg:yellow]]¶[[fg:gray]]│[[end]]{}", msg).as_str());
				msglog.tell_planq(" ");
			}
			PlanqCmd::Help => {
				msglog.tell_planq("[[fg:yellow]]¶[[fg:gray]]│[[end]]Available commands:");
				for command in PlanqCmd::iter() {
					msglog.tell_planq(format!("[[fg:yellow]]¶[[fg:gray]]│[[end]]  {}", command).as_str());
				}
				msglog.tell_planq(" ");
			}
			PlanqCmd::Shutdown => { todo!(); /* trigger a shutdown */ }
			PlanqCmd::Reboot => { todo!(); /* execute a reboot */ }
			PlanqCmd::Connect(_target) => { todo!(); /* run the planq.connect subroutine */ }
			PlanqCmd::Disconnect => { todo!(); /* run the planq.disconnect subroutine */ }
			_ => { /* NoOperation */ }
		}
		false
	}
}

//  ###: ENGINE SYSTEMS
/// Watches for a Resource of type Saved, and when found, emits a QuitRequest Event; set to run after moonshine's save system
fn raise_quit_event_after_saving_game(mut quit_events: EventWriter<QuitRequest>,
																			saved_data: Option<Res<Saved>>
) {
	debug!("* raise_quit_event_after... running");
	if let Some(data) = saved_data {
		if data.is_added() {
			eprintln!("* Raising QuitRequest via raise_quit_event_after...");
			quit_events.send(QuitRequest);
		}
	} else {
		debug!("* Did not locate a Resource of type Saved");
	}
}
/// Watches for a new Resource of type Loaded, and when found, emits a StartRequest Event; set to run after moonshine's load system
fn raise_start_event_after_loading_game(mut start_events: EventWriter<StartRequest>,
																				loaded_data: Option<Res<Loaded>>,
																				mut _model: ResMut<WorldModel>,
																				_b_query: Query<(Entity, &Body)>
) {
	debug!("* raise_start_event_after... running");
	if let Some(data) = loaded_data {
		if data.is_added() {
			eprintln!("* A loaded game was detected, now starting");
			//let enty_body_map = b_query.iter().map(|pair| (pair.0, pair.1.extent.clone())).collect();
			//model.reload_tile_contents(data.entity_map.clone(), enty_body_map); // DEBUG: moved this to a startup system
			start_events.send(StartRequest);
		}
	} else {
		debug!("* Did not find a Loaded resource");
	}
}

//  ###: SIMPLE TYPES AND HELPERS
//   ##: EngineMode
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
//   ##: AppResult
/// Application result type, provides some nice handling if the game crashes
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;
//   ##: Request Events
//  Defines a set of 'global' requests that can be made via Bevy's event system for certain operations
//    #: LoadRequest
#[derive(Event)]
pub struct LoadRequest {
	pub path: PathBuf
}
impl LoadFromFileRequest for LoadRequest {
	fn path(&self) -> &Path {
		self.path.as_ref()
	}
}
//    #: QuitRequest
#[derive(Event)]
pub struct QuitRequest;
//    #: SaveRequest
#[derive(Event)]
pub struct SaveRequest {
	pub path: PathBuf
}
impl SaveIntoFileRequest for SaveRequest {
	fn path(&self) -> &Path {
		self.path.as_ref()
	}
}
//    #: StartRequest
#[derive(Event)]
pub struct StartRequest;

// EOF
