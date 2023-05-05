// app/mod.rs
// generated as app.rs using orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use std::error;
use bevy_save::prelude::*;
use bevy::app::App;
use bracket_rex::prelude::XpFile;
use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Rect, Layout, Direction, Constraint};
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Clear, List, ListItem};
pub mod handler;
pub mod event;
pub mod viewport;
pub mod planq;
pub mod tui;
pub mod messagelog;
pub mod image_loader;
pub mod menu;
use viewport::Viewport;
use crate::app::planq::*;
use crate::app::messagelog::MessageLog;
use crate::app::image_loader::load_rex_pgraph;
use crate::app::menu::{MainMenuItems, MenuSelector};
use crate::item_builders::{ItemBuilder, ItemType};
use crate::components::*;
use crate::components::{Name, GameEventType};
use crate::camera_system::CameraView;
use bevy::ecs::entity::*;
use bevy::prelude::*;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;
/// Contains all of the coordination and driver logic for the game itself
pub struct GameEngine {
	pub running: bool, // control flag for the game loop as started in main()
	pub standby: bool, // if true, the game itself is not yet loaded/has ended
	pub mode: EngineMode, // sets the engine's runtime context: paused, item selection, &c
	pub app: App, // bevy::app::App, contains all of the ECS bits
	pub artificer: ItemBuilder,
	pub recalculate_layout: bool,
	pub ui_grid: UIGrid,
	pub main_menu_is_visible: bool,
	pub main_menu: MenuSelector<MainMenuItems>,
	pub item_chooser_is_visible: bool,
	pub item_chooser: MenuSelector<Entity>, // provides the menu for item pickup from World
	pub target_chooser_is_visible: bool,
	pub target_chooser: MenuSelector<Entity>, // provides the menu for target selection from World
	// see the planq obj for the planq_chooser's visibility setting
	pub planq_chooser: MenuSelector<Entity>, // provides a generic menu selector via the Planq
	pub player_action: GameEventType, // in practice only a subset of types will be used
}
impl GameEngine {
	/// Constructs a new instance of [`GameEngine`].
	pub fn new(max_area: Rect) -> Self {
		let mut new_eng = Self {
			// Set standby to true and main_menu_is_visible to true to restore the proto-start screen
			running: true,
			standby: false,
			mode: EngineMode::Running,
			app: App::new(),
			artificer: ItemBuilder { spawn_count: 0 },
			recalculate_layout: false,
			ui_grid: UIGrid::new(), // Can't be a Bevy Resource because tui::Rect is ineligible
			main_menu_is_visible: false,
			main_menu: MenuSelector::with_items(Vec::new()),
			item_chooser_is_visible: false,
			item_chooser: MenuSelector::with_items(Vec::new()),
			target_chooser_is_visible: false,
			target_chooser: MenuSelector::with_items(Vec::new()),
			planq_chooser: MenuSelector::with_items(Vec::new()),
			player_action: GameEventType::NullEvent,
		};
		new_eng.ui_grid.calc_layout(max_area);
		return new_eng;
	}
	/// Runs a single update cycle of the game state
	pub fn tick(&mut self) {
		//eprintln!("TICK"); // DEBUG:
		self.app.update();
	}
	/// Renders the main menu, useful so that we can draw it by itself in standby mode
	pub fn render_main_menu<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		// self.main_menu.list is what holds the backing values
		// this mm_items list holds the matching list of display values
		let mut mm_items = Vec::new();
		for item in MainMenuItems::to_list().iter() {
			match item {
				MainMenuItems::NULL => { /* do nothing, ofc */ }
				MainMenuItems::NEWGAME => {
					mm_items.push(ListItem::new(item.to_string()));
					self.main_menu.list.push(*item);
				}
				MainMenuItems::LOADGAME => { /* FIXME: only add LOADGAME if a save exists */ }
				MainMenuItems::SAVEGAME => { /* FIXME: only add SAVEGAME if a game is going */ }
				MainMenuItems::QUIT => {
					mm_items.push(ListItem::new(item.to_string()));
					self.main_menu.list.push(*item);
				}
			}
		}
		let menu = List::new(mm_items)
			.block(Block::default().title("Main Menu").borders(Borders::ALL))
			.style(Style::default().fg(Color::White).bg(Color::Black))
			.highlight_style(Style::default().fg(Color::Black).bg(Color::White))
			.highlight_symbol("->");
		let area = Rect::new(10, 12, 23, 10); // WARN: magic numbers
		frame.render_widget(Clear, area);
		frame.render_stateful_widget(menu, area, &mut self.main_menu.state);
	}
	/// Renders the game and its GUI.
	pub fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		// If the engine is still in standby mode, defer to that immediately
		if self.standby { self.render_main_menu(frame); return; }
		// METHOD
		// - if the layout is 'dirty', recalculate it
		if self.recalculate_layout {
			self.calc_layout(frame.size());
			self.recalculate_layout = false;
		}
		// ui_grid index list:
		// 0: Viewport -> CameraView_main
		// 1: (Planq output)
		// 2: (Status bars)
		// Start by drawing the output of the main view
		// If there's a valid CameraView to render, use that
		if let Some(view) = self.app.world.get_resource_mut::<CameraView>() {
			frame.render_widget(
				Viewport::new(&view).block(
					Block::default()
					.borders(Borders::NONE)
					.border_type(BorderType::Double)
					.border_style(Style::default().fg(Color::White)),
				),
				self.ui_grid.camera_main,
			);
		} else { // otherwise, just show a blank screen
			frame.render_widget(
				Block::default()
				.title("[no CameraView initialized]")
				.borders(Borders::ALL)
				.border_type(BorderType::Double)
				.border_style(Style::default().fg(Color::White)),
				self.ui_grid.camera_main,
			);
		}
		// Render the main message log pane
		// Obtain a slice of the message log here and feed to the next widget
		let msglog_ref = self.app.world.get_resource::<MessageLog>();
		if msglog_ref.is_some() {
			let msglog = msglog_ref.unwrap(); // get a handle on the msglog service
			let worldmsg = msglog.get_log("world".to_string()); // get the full backlog
			//eprintln!("*** worldmsg.len {}, ui_grid.msg_world.height {}", worldmsg.len() as i32, self.ui_grid.msg_world.height as i32); // DEBUG:
			/* FIXME: magic number offset for window borders
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
		// Draw the PLANQ
		let ppos = self.app.world.get_resource::<Position>().unwrap(); // DEBUG:
		let mut planq_text = vec!["test string".to_string()]; // DEBUG:
		planq_text.push(format!("*D* x: {}, y: {}, z: {}", ppos.x, ppos.y, ppos.z)); // DEBUG:
		// FIXME: only draw the regular Planq bar if the Planq is actually on the player and running
		let planq = self.app.world.get_resource::<PlanqSettings>().unwrap();
		if planq.is_carried {
			// Always draw the Planq's status output
			frame.render_widget(
				// if planq.is_running ... TODO:
				PlanqStatus::new(&planq_text).block(
					Block::default()
					.title("PLANQ OUTPUT")
					.title_alignment(Alignment::Center)
					.borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
					.border_type(BorderType::Thick)
					.border_style(Style::default().fg(Color::White)),
				),
				self.ui_grid.planq_status,
			);
			// Display output #1 if enabled
			if planq.output_1_enabled {
				// match planq.output_1_mode { ... (build an enum?) TODO:
				if planq.show_inventory {
					if planq.inventory_list.len() > 0 {
						let mut item_list = Vec::new();
						self.planq_chooser.list.clear();
						for item in &planq.inventory_list {
							self.planq_chooser.list.push(*item);
							let mut name = self.app.world.get::<Name>(*item).unwrap().name.clone();
							name.push_str(&String::from(format!("-{item:?}")));
							item_list.push(ListItem::new(name.clone()));
						}
						let inventory_menu = List::new(item_list)
							.block(Block::default().title("Inventory").borders(Borders::ALL))
							.style(Style::default())
							.highlight_style(Style::default().fg(Color::Black).bg(Color::White))
							.highlight_symbol("->");
						frame.render_stateful_widget(inventory_menu, self.ui_grid.planq_output_1, &mut self.planq_chooser.state);
					} else {
						frame.render_widget(
							Paragraph::new("inventory is empty").block(
								Block::default()
								.borders(Borders::ALL)
								.border_type(BorderType::Thick)
								.border_style(Style::default().fg(Color::White)),
							),
							self.ui_grid.planq_output_1,
						);
					}
				}
			}
			// Display output #2 if enabled
			if planq.output_2_enabled {
				// TODO: figure out which output to display here
				frame.render_widget(
					Block::default()
					.title("output_2 test")
					.title_alignment(Alignment::Left)
					.borders(Borders::ALL)
					.border_type(BorderType::Thick)
					.border_style(Style::default().fg(Color::White)),
					self.ui_grid.planq_output_2,
				);
			}
		}
		// Display some kind of 'planq offline' state if not carried
		else { // Player is not carrying a planq
			frame.render_widget(
				Paragraph::new("\n\n no PLANQ detected ").block(
					Block::default()
					.borders(Borders::NONE)
					.border_type(BorderType::Thick)
					.border_style(Style::default().fg(Color::Gray).bg(Color::Black))
				),
				self.ui_grid.planq_status,
			);
		}
		// Render any optional menus and layers, ie main menu
		if self.main_menu_is_visible {
			/*
			self.main_menu.list = MainMenuItems::to_list(); // produces Vec<MainMenuItems>
			let mut mm_items = Vec::new();
			for item in self.main_menu.list.iter() {
				mm_items.push(ListItem::new(item.to_string()));
			}
			let menu = List::new(mm_items)
				.block(Block::default().title("Main Menu").borders(Borders::ALL))
				.style(Style::default().fg(Color::White).bg(Color::Black))
				.highlight_style(Style::default().fg(Color::Black).bg(Color::White))
				.highlight_symbol("->");
			let area = Rect::new(10, 12, 23, 10); // WARN: magic numbers
			frame.render_widget(Clear, area);
			frame.render_stateful_widget(menu, area, &mut self.main_menu.state);
			*/
			self.render_main_menu(frame);
			/* this fires on every index change, not just confirmation
			match self.main_menu.state.selected() {
				None => { }
				Some(selection) => {eprintln!("sel: {}", selection);} // DEBUG:
			}
			*/
		}
		else if self.item_chooser_is_visible {
			let mut item_list = Vec::new();
			for item in self.item_chooser.list.iter() {
				let name = self.app.world.get::<Name>(*item);
				item_list.push(ListItem::new(name.unwrap().name.clone()));
			}
			let menu = List::new(item_list)
				.block(Block::default().title("Select:").borders(Borders::ALL))
				.style(Style::default())
				.highlight_style(Style::default().fg(Color::Black).bg(Color::White))
				.highlight_symbol("->");
			let area = Rect::new(40, 12, 23, 10); // WARN: magic numbers
			frame.render_widget(Clear, area);
			frame.render_stateful_widget(menu, area, &mut self.item_chooser.state);
		}
		else if self.target_chooser_is_visible {
			let mut target_list = Vec::new();
			for target in self.target_chooser.list.iter() {
				let name = self.app.world.get::<Name>(*target);
				target_list.push(ListItem::new(name.unwrap().name.clone()));
			}
			let menu = List::new(target_list)
				.block(Block::default().title("Target:").borders(Borders::ALL))
				.style(Style::default())
				.highlight_style(Style::default().fg(Color::Black).bg(Color::White))
				.highlight_symbol("->");
			let area = Rect::new(40, 12, 23, 10); // WARN: magic numbers, see above as well
			frame.render_widget(Clear, area);
			frame.render_stateful_widget(menu, area, &mut self.target_chooser.state);
			// FIXME: find a way to draw a target reticle here
		}
		// Display the fancy "PAUSED" banner if the game is paused
		// FIXME: this should probably just use the eng.mode property to avoid making a world call
		let eng_settings = self.app.world.get_resource::<GameSettings>().unwrap();
		if eng_settings.mode == EngineMode::Paused {
			let xpfile = &XpFile::from_resource("../resources/big_pause.xp").unwrap();
			let graphic = load_rex_pgraph(xpfile);
			let banner_area = Rect::new(10, 5, graphic.width() as u16, (graphic.height() + 2) as u16);
			let banner_img = Paragraph::new(graphic).block(Block::default().borders(Borders::TOP | Borders::BOTTOM));
			frame.render_widget(Clear, banner_area);
			frame.render_widget(banner_img, banner_area);
		} else if eng_settings.mode == EngineMode::GoodEnd {
			eprintln!("*************************");
			eprintln!("*** Victory detected! ***");
			eprintln!("*************************");
			self.quit();
		}
	}
	/// Toggles the main menu's visibility each time it is called
	pub fn main_menu_toggle(&mut self) {
		// sets the visibility state of the main menu popup
		if self.main_menu_is_visible == false { self.main_menu_is_visible = true; }
		else { self.main_menu_is_visible = false; }
	}
	/// Shows the item chooser menu
	pub fn show_item_chooser(&mut self) { self.item_chooser_is_visible = true; }
	/// Hides the item chooser menu
	pub fn hide_item_chooser(&mut self) { self.item_chooser_is_visible = false; }
	/// Shows the targeting menu
	pub fn show_target_chooser(&mut self) { self.target_chooser_is_visible = true; }
	/// Hides the targeting menu
	pub fn hide_target_chooser(&mut self) { self.target_chooser_is_visible = false; }
	/// Requests a recalculation of the GameEngine.ui_grid object based on the given area
	pub fn calc_layout(&mut self, area: Rect) {
		//eprintln!("calc_layout() called"); // DEBUG:
		self.ui_grid.calc_layout(area);
		let camera_ref = self.app.world.get_resource_mut::<CameraView>();
		if camera_ref.is_some() {
			eprintln!("- resizing cameraview during call to calc_layout()");// DEBUG:
			let mut camera = camera_ref.unwrap();
			camera.set_dims(self.ui_grid.camera_main.width as i32, self.ui_grid.camera_main.height as i32);
		}
	}
	/// Handles a call to stop the game and exit
	pub fn quit(&mut self) {
		self.running = false;
	}
	/// Changes the pause-state of the game, ie transition between Running/Paused modes
	pub fn pause_game(&mut self, state: bool) {
		if state {
			self.mode = EngineMode::Paused;
		} else {
			self.mode = EngineMode::Running;
		}
		// FIXME: dispatch event?
	}
	/// Toggles between Running/Paused depending on last mode
	pub fn pause_toggle(&mut self) {
		if self.mode == EngineMode::Paused {
			self.mode = EngineMode::Running;
		} else {
			self.mode = EngineMode::Paused;
		}
		// FIXME: dispatch event?
	}
	/// Handles a call to save the game
	pub fn save_game(&mut self) {
	//  WARN: By default (not sure how to change this!), on Linux, this savegame will be at
	//      ~/.local/share/spacegame/saves/FILENAME.sav
		//eprintln!("SAVEGAME called"); // DEBUG:
		self.app.world.save("savegame");
	}
	/// Handles a call to load the game
	pub fn load_game(&mut self) {
		//eprintln!("LOADGAME called"); // DEBUG:
		self.app.world.load("savegame");
		// FIXME: need to set the player's viewshed to dirty HERE to force a viewport update
	}
	/// Creates an item [TODO: and returns a ref to it for further customization]
	pub fn make_item(&mut self, new_type: ItemType, location: Position) {
		self.artificer.spawn(&mut self.app.world, new_type, location);
	}
	/// Sets the engine's mode; requires the event controller so it can dispatch a game event
	pub fn set_mode(&mut self, new_mode: EngineMode) {
		self.mode = new_mode; // Update the setting at the outer layer
		// Dispatch an event through the inner layers
		let game_events: &mut Events<GameEvent> = &mut self.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
		game_events.send(GameEvent::new(GameEventType::ModeSwitch(new_mode), None));
	}

}

#[derive(Resource, FromReflect, Reflect, Copy, Clone, PartialEq, Eq, Default)]
#[reflect(Resource)]
pub struct GameSettings {
	pub mode: EngineMode,
}
impl GameSettings {
	pub fn new() -> GameSettings {
		GameSettings {
			mode: EngineMode::Running,
		}
	}
}

/// Provides a bunch of named fields (rather than a tuple) of grid components
pub struct UIGrid {
	/// Provides the main view onto the worldmap
	pub camera_main:    Rect,
	/// Designates the 'default' message log, which always shows msgs from the World channel
	pub msg_world:      Rect,
	/// Designates the area for the whole Planq sidebar, all panels included
	pub planq_sidebar:  Rect,
	/// Designates the space reserved for the Planq's stats: offline status, battery power, &c
	pub planq_status:   Rect,
	/// Designates the first output screen of the Planq; user-configurable
	pub planq_output_1: Rect,
	/// Designates the second output screen of the Planq; user-configurable
	pub planq_output_2: Rect,
}
impl UIGrid {
	pub fn new() -> UIGrid {
		UIGrid {
			camera_main: Rect::default(),
			msg_world: Rect::default(),
			planq_sidebar: Rect::default(),
			planq_status: Rect::default(),
			planq_output_1: Rect::default(),
			planq_output_2: Rect::default(),
		}
	}
	/// Recalculates the UI layout based on the given size
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
		let main_horiz_split = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Min(30), Constraint::Length(30)].as_ref())
			.split(max_area).to_vec();
		let camera_worldmsg_split = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(30), Constraint::Length(12)].as_ref())
			.split(main_horiz_split[0]).to_vec();
		let planq_splits = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(3), Constraint::Length(20), Constraint::Length(20)].as_ref())
			.split(main_horiz_split[1]).to_vec();
		// Update the UIGrid itself to hold the new sizes
		self.camera_main = camera_worldmsg_split[0];
		self.msg_world = camera_worldmsg_split[1];
		self.planq_sidebar = main_horiz_split[1];
		self.planq_status = planq_splits[0];
		self.planq_output_1 = planq_splits[1];
		self.planq_output_2 = planq_splits[2];
	}
}

// EOF
