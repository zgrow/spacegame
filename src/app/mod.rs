// app/mod.rs
// generated as app.rs using orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use std::error;
use bracket_rex::prelude::XpFile;
use bevy::app::App;
use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Rect, Layout, Direction, Constraint};
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Clear, List, ListItem};
pub mod handler;
pub mod event;
pub mod viewport;
pub mod tui;
pub mod messagelog;
pub mod image_loader;
pub mod menu;
use viewport::Viewport;
use crate::app::messagelog::MessageLog;
use crate::app::image_loader::load_rex_pgraph;
use crate::app::menu::{MainMenuItems, MenuSelector};
use crate::components::{Position, Player, CameraView};
use crate::map::Map;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;
/// Contains all of the coordination and driver logic for the game itself
pub struct GameEngine<'a> {
	pub running: bool, // running vs stopped
	pub paused: bool, // paused vs unpaused
	pub app: App, // bevy::app::App, contains all of the ECS bits
	pub recalculate_layout: bool,
	pub ui_grid: Vec<Rect>,
	pub player: Player,
	pub show_main_menu: bool,
	//pub sel_main_menu: ListState,
	pub main_menu: MenuSelector<ListItem<'a>>,
}
impl<'a> GameEngine<'a> {
	/// Constructs a new instance of [`GameEngine`].
	//pub fn new(layout: Vec<Rect>) -> Self {
	pub fn new(max_area: Rect) -> Self {
		let mut new_eng = Self {
			running: true,
			paused: false,
			app: App::new(),
			recalculate_layout: false,
			ui_grid: Vec::new(), // Can't be a Bevy Resource because tui::Rect is ineligible
			player: Player::default(),
			show_main_menu: false,
			main_menu: MenuSelector::with_items(Vec::new()),
		};
		new_eng.calc_layout(max_area);
		return new_eng;
	}
	/// Runs a single update cycle of the game state
	pub fn tick(&mut self) {
		eprintln!("TICK");
	}
	/// Renders the user interface widgets.
	pub fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		// METHOD
		// - if the layout is 'dirty', recalculate it
		if self.recalculate_layout {
			self.calc_layout(frame.size());
			self.recalculate_layout = false;
		}
		/* Use the layout to build up the UI and its contents
		 * - iterate through the layout stack
		 * - if the object indexed to the layout Rect is active, then draw it
		 * frame.render_widget(self, Widget, area: Rect)
		 * - might consider nesting the calls:
		 *   draw_thing<Backend>(f: &mut Frame<Backend>, app: &mut App, area: Rect)
		 * TODO: one day i'll have the time to make this dynamic/rearrangable...
		 *        right now we're just going to use a hardcoded set and order
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
		// ui_grid index list:
		// 0: Viewport -> CameraView_main
		// 1: (Planq output)
		// 2: (Status bars)
		// Start by drawing the output of the main view
		frame.render_widget(
			Viewport::new(&self.app)
			.block(
				Block::default()
					.borders(Borders::ALL)
					.border_type(BorderType::Double)
					.border_style(Style::default().fg(Color::White)),
			),
			self.ui_grid[0],
		);
		// Render the main message log pane
		// Obtain a slice of the message log here and feed to the next widget
		let mut log_text = "--no logs found--".to_string();
		let msglog_ref = self.app.world.get_resource::<MessageLog>();
		if msglog_ref.is_some() {
			let msglog = msglog_ref.unwrap();
			let worldmsg = msglog.get_log("world".to_string());
			if !worldmsg.is_empty() { log_text = worldmsg[0].clone(); }
		}
		// Draw the message log pane
		frame.render_widget(
			Paragraph::new(log_text) // requires a Vec<Spans<'a>> for group insert on creation
			.block(
				Block::default()
					.title("PLANQ: -offline- ")
					.title_alignment(Alignment::Left)
					.borders(Borders::ALL)
					.border_type(BorderType::Thick)
					.border_style(Style::default().fg(Color::White)),
			),
			self.ui_grid[1],
		);
		// Draw the sidebar pane
		frame.render_widget(
			Block::default()
				.title("Status Rack")
				.title_alignment(Alignment::Center)
				.borders(Borders::ALL)
				.border_type(BorderType::Thick)
				.border_style(Style::default().fg(Color::White)),
			self.ui_grid[2],
		);
		// Render any optional menus and layers, ie main menu
		if self.show_main_menu {
			//let main_menu_list = vec![ListItem::new("Alpha"), ListItem::new("Beta"), ListItem::new("Gamma")];
			self.main_menu.list = MainMenuItems::to_list();
			let menu = List::new(&*self.main_menu.list)
				.block(Block::default().title("Main Menu").borders(Borders::ALL))
				.style(Style::default().fg(Color::White).bg(Color::Black))
				.highlight_style(Style::default().fg(Color::Black).bg(Color::White))
				.highlight_symbol("->");
			let area = Rect::new(10, 12, 23, 10); // NOTE: magic numbers
			frame.render_widget(Clear, area);
			frame.render_stateful_widget(menu, area, &mut self.main_menu.state);
			/* this fires on every index change, not just confirmation
			match self.main_menu.state.selected() {
				None => { }
				Some(selection) => {eprintln!("sel: {}", selection);}
			}
			*/
		}
		if self.paused {
			let xpfile = &XpFile::from_resource("../resources/big_pause.xp").unwrap();
			let graphic = load_rex_pgraph(xpfile);
			let banner_area = Rect::new(10, 5, graphic.width() as u16, (graphic.height() + 2) as u16);
			let banner_img = Paragraph::new(graphic).block(Block::default().borders(Borders::TOP | Borders::BOTTOM));
			frame.render_widget(Clear, banner_area);
			frame.render_widget(banner_img, banner_area);
		}
	}
	/// Returns true if the specified Position is occupied by a piece of furniture, an entity, etc
	pub fn is_occupied(&self, target: Position) -> bool {
		// Is there an entity at this spot?
		// for all entities with a Position,
		//  return true if enty.posn matches target
		// Is there a wall at this spot?
		// FIXME: does not handle entity collision!
		let map = self.app.world.get_resource::<Map>().unwrap();
		return map.is_occupied(target);
	}
	/// Toggles the paused state of the game engine when called
	pub fn pause_toggle(&mut self) {
		if self.paused == true { self.paused = false; }
		else { self.paused = true; }
	}
	/// Toggles the main menu's visibility each time it is called
	pub fn main_menu_toggle(&mut self) {
		// sets the visibility state of the main menu popup
		if self.show_main_menu == false { self.show_main_menu = true; }
		else { self.show_main_menu = false; }
	}
	/// Recalculates the GameEngine.ui_grid object based on the given area
	pub fn calc_layout(&mut self, area: Rect) {
		//eprintln!("calc_layout() called"); // DEBUG:
		let mut first_split = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Min(30), Constraint::Length(30)].as_ref())
			.split(area).to_vec();
		self.ui_grid = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(30), Constraint::Length(12)].as_ref())
			.split(first_split[0]).to_vec();
		self.ui_grid.push(first_split.pop().unwrap());
		let camera_ref = self.app.world.get_resource_mut::<CameraView>();
		if camera_ref.is_some() {
			eprintln!("- resizing cameraview during call to calc_layout()");// DEBUG:
			let mut camera = camera_ref.unwrap();
			camera.set_dims(self.ui_grid[0].width as i32, self.ui_grid[0].height as i32);
		}
	}
}

// EOF
