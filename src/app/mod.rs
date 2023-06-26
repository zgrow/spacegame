// app/mod.rs
// generated as app.rs using orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use std::error;
use bevy_save::prelude::*;
use bevy::app::App;
use bracket_rex::prelude::XpFile;
use ratatui::backend::Backend;
use ratatui::layout::{Rect, Layout, Direction, Constraint};
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Clear, List, ListItem};
pub mod handler;
pub mod tui_event;
pub mod viewport;
pub mod planq;
pub mod tui;
pub mod messagelog;
pub mod image_loader;
pub mod menu;
pub mod event;
use viewport::Viewport;
use crate::app::planq::*;
use crate::app::messagelog::MessageLog;
use crate::app::image_loader::load_rex_pgraph;
use crate::app::menu::{MainMenuItems, MenuSelector};
use crate::app::event::{GameEvent, GameEventType};
use crate::item_builders::{ItemBuilder, ItemType};
use crate::components::*;
use crate::components::Name;
use crate::camera_system::CameraView;
use bevy::ecs::entity::*;
use bevy::prelude::*;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;
/// Contains all of the coordination and driver logic for the game itself
pub struct GameEngine<'a> {
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
	pub planq_stdin: PlanqInput<'a>,
}
impl GameEngine<'_> {
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
			planq_stdin: PlanqInput::new(),
		};
		new_eng.ui_grid.calc_layout(max_area);
		new_eng
	}
	/// Runs a single update cycle of the game state
	pub fn tick(&mut self) {
		//eprintln!("TICK"); // DEBUG:
		self.app.update();
		// check for any mode switches propagating up from a game event
		let mut settings = self.app.world.get_resource_mut::<GameSettings>().unwrap();
		if settings.mode_changed {
			self.mode = settings.mode;
			settings.mode_changed = false;
		}
	}
	/// Renders the main menu, useful so that we can draw it by itself in standby mode
	pub fn render_main_menu<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		// self.main_menu.list is what holds the backing values
		// this mm_items list holds the matching list of display values
		let mut mm_items = Vec::new();
		self.main_menu.list.clear();
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
	/// Renders the PLANQ sidebar object
	pub fn render_planq<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		let mut planq = self.app.world.get_resource_mut::<PlanqData>().unwrap();
		// TODO: optimize this to only fire if the number of status bars actually changes
		self.ui_grid.p_status_height = planq.status_bars.len(); // WARN: assumes all status bars are h = 1!
		self.ui_grid.calc_planq_layout(self.ui_grid.planq_sidebar);
		// Display some kind of 'planq offline' state if not carried
		// TODO: replace the 'no planq detected' message with something nicer
		if !planq.is_carried { // Player is not carrying a planq
			frame.render_widget(
				Paragraph::new("\n\n[no PLANQ detected] ").block(
					Block::default().borders(Borders::NONE)
				),
				self.ui_grid.planq_status,
			);
			return;
		}
		// TODO: replace the 'planq offline' message with something nicer
		// make sure it includes a battery readout: charge level, "NO BATT", &c
		else if planq.cpu_mode == PlanqCPUMode::Offline {
			frame.render_widget(
				Paragraph::new("\n\n[PLANQ offline]").block(
					Block::default()
					.borders(Borders::ALL)
					.border_type(BorderType::Thick)
					.border_style(Style::default().fg(Color::Gray).bg(Color::Black))
				),
				self.ui_grid.planq_status,
			);
			return;
		}
		// Always render the status widgets if there's power
		planq.render_status_bars(frame, self.ui_grid.planq_status);
		if planq.show_terminal {
			planq.render_terminal(frame, self.ui_grid.planq_stdout);
			// Only display the CLI if there's a terminal visible to contain it
			if planq.show_cli_input {
				planq.render_cli(frame, self.ui_grid.planq_stdin, &mut self.planq_stdin);
			}
		}
		//if planq.output_1_enabled {
		//	planq.render_planq_stdout_1(frame, self.ui_grid.planq_output_1);
		//}
		//if planq.output_2_enabled {
		//	planq.render_planq_stdout_2(frame, self.ui_grid.planq_output_2);
		//}
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
		let msglog = msglog_ref.unwrap_or_default(); // get a handle on the msglog service
		if msglog_ref.is_some() {
			let worldmsg = msglog.get_log_as_spans("world".to_string(), 0); // get the full backlog
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
		self.render_planq(frame); // The PLANQ can decide what it needs to render or not
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
			// TODO: find a way to draw a target reticle here
		}
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
	/// Toggles the main menu's visibility each time it is called
	pub fn main_menu_toggle(&mut self) {
		// sets the visibility state of the main menu popup
		if !self.main_menu_is_visible { self.main_menu_is_visible = true; }
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
	/// Shows the PLANQ's cli input if it's running, &c
	pub fn show_planq_cli(&mut self) {

	}
	/// Hides the PLANQ's cli
	pub fn hide_planq_cli(&mut self) { /* this can always be executed */ }
	/// Requests a recalculation of the GameEngine.ui_grid object based on the given area
	pub fn calc_layout(&mut self, area: Rect) {
		//eprintln!("calc_layout() called"); // DEBUG:
		self.ui_grid.calc_layout(area);
		let camera_ref = self.app.world.get_resource_mut::<CameraView>();
		if let Some(mut camera) = camera_ref {
		//if camera_ref.is_some() {
			eprintln!("- resizing cameraview during call to calc_layout()");// DEBUG:
			//let mut camera = camera_ref.unwrap();
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
			self.set_mode(EngineMode::Paused);
		} else {
			self.set_mode(EngineMode::Running);
		}
	}
	/// Toggles between Running/Paused depending on last mode
	pub fn pause_toggle(&mut self) {
		if self.mode == EngineMode::Paused {
			self.pause_game(false);
		} else {
			self.pause_game(true);
		}
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
	pub mode_changed: bool,
}
impl GameSettings {
	pub fn new() -> GameSettings {
		GameSettings {
			mode: EngineMode::Running,
			mode_changed: false,
		}
	}
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
	pub camera_main:    Rect,
	/// Designates the 'default' message log, which always shows msgs from the World channel
	pub msg_world:      Rect,
	/// Designates the area for the whole Planq sidebar, all panels included
	pub planq_sidebar:  Rect,
	/// Designates the space reserved for the Planq's stats: offline status, battery power, &c
	pub planq_status:   Rect,
	/// Designates the space for the Planq's entire terminal
	pub planq_screen:    Rect,
	/// Designates the output screen of the Planq
	pub planq_stdout: Rect,
	/// Designates the CLI input of the Planq
	pub planq_stdin: Rect,
	/// Sets the height of the planq_status widget, will be updated during gameplay
	pub p_status_height: usize,
	/// Sets the height of the planq's CLI widget
	pub p_stdin_height: usize
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
		// (as a method call somewhere else, so that it can be redone outside of here
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
		// Split the entire window between (1/2)[0] and (3)[1] horizontally
		let main_horiz_split = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Min(30), Constraint::Length(32)].as_ref())
			.split(max_area).to_vec();
		// Split (1)[0] and (2)[1] vertically
		let camera_worldmsg_split = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(30), Constraint::Length(12)].as_ref())
			.split(main_horiz_split[0]).to_vec();
		// OLD METHOD
		// Split (3) into the PLANQ output sizes: (status)[0], (stdout_1)[1], (stdout_2)[2], as a vertical stack
		//let planq_splits = Layout::default()
		//	.direction(Direction::Vertical)
		//	.constraints([Constraint::Min(3), Constraint::Length(22), Constraint::Length(22)].as_ref())
		//	.split(main_horiz_split[1]).to_vec();
		// Split (planq_splits)[0] vertically to provide a height=1 area for the PLANQ's (CLI input)[1]
		//let planq_status = Layout::default()
		//	.direction(Direction::Vertical)
		//	.constraints([Constraint::Min(1), Constraint::Max(1)].as_ref())
		//	.split(planq_splits[0]).to_vec();
		//  ****
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
