// app/mod.rs
// generated as app.rs using orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use std::error;
use bevy::app::App;
use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Rect, Layout, Direction, Constraint};
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Clear};
pub mod handler;
pub mod event;
pub mod viewport;
pub mod tui;
pub mod messagelog;
use viewport::Viewport;
use crate::app::messagelog::MessageLog;
use crate::components::{Position, Player};
use crate::map::Map;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;
/// Contains all of the coordination and driver logic for the game itself
#[derive(Debug)]
pub struct GameEngine {
	/// Is the application running?
	pub running: bool,
	pub app: App, // bevy::app::App, contains all of the ECS bits
	pub recalculate_layout: bool,
	pub ui_grid: Vec<Rect>,
	pub player: Player,
	pub show_main_menu: bool,
}
impl GameEngine {
	/// Constructs a new instance of [`GameEngine`].
	pub fn new(layout: Vec<Rect>) -> Self {
		Self {
			running: true,
			app: App::new(),
			recalculate_layout: true,
			ui_grid: layout, // Can't be a Bevy Resource because tui::Rect is ineligible
			player: Player::default(),
			show_main_menu: false,
		}
	}
	/// Recalculates the UI layout based on the widget sizes
	pub fn calc_layout(&self, new_width: i32, new_height: i32) -> Vec<Rect> {
		Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Min(30)].as_ref())
			.split(Rect {
				x: 0,
				y: 0,
				width: new_width as u16,
				height: new_height as u16,
			})
			.to_vec()
	}
	/// Runs a single update cycle of the game state
	pub fn tick(&self) {
		eprintln!("TICK");
	}
	/// Renders the user interface widgets.
	pub fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		// METHOD
		// - if the layout is 'dirty', recalculate it
		if self.recalculate_layout {
			// FIXME: this logic is duplicated in main()! if this is changed, change there also
			let mut first_split = Layout::default()
				.direction(Direction::Horizontal)
				.constraints([Constraint::Min(30), Constraint::Length(30)].as_ref())
				.split(frame.size()).to_vec();
			self.ui_grid = Layout::default()
				.direction(Direction::Vertical)
				.constraints([Constraint::Min(30), Constraint::Length(12)].as_ref())
				.split(first_split[0]).to_vec();
			self.ui_grid.push(first_split.pop().unwrap());
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
		// Obtain a slice of the message log here and feed to the next widget
		let msglog_ref = self.app.world.get_resource::<MessageLog>();
		if msglog_ref.is_some() {
			let msglog = msglog_ref.unwrap();
			let worldmsg = msglog.get_log("world-X".to_string());
			let mut log_text = "--no logs found--".to_string();
			if !worldmsg.is_empty() { log_text = worldmsg[0].clone(); }
			frame.render_widget(
				Paragraph::new(log_text) // requires a Vec<Spans<'a>> for group insert on creation
				.block(
					Block::default()
						.title("PLANQ: -offline-")
						.title_alignment(Alignment::Left)
						.borders(Borders::ALL)
						.border_type(BorderType::Thick)
						.border_style(Style::default().fg(Color::White)),
				),
				self.ui_grid[1],
			);
		}
		if self.show_main_menu {
			let block = Block::default().title("Test Menu").borders(Borders::ALL);
			let area = Rect::new(10, 10, 10, 10); // NOTE: magic numbers
			frame.render_widget(Clear, area);
			frame.render_widget(block, area);
		}
	}
	/// Returns true if the specified Position is occupied by a piece of furniture, an entity, etc
	pub fn is_occupied(&self, target: Position) -> bool {
		// Is there an entity at this spot?
		// for all entities with a Position,
		//  return true if enty.posn matches target
		// Is there a wall at this spot?
		let map = self.app.world.get_resource::<Map>().unwrap();
		return map.is_occupied(target);
	}
	/// Toggles the main menu's visibility each time it is called
	pub fn main_menu_toggle(&mut self) {
		// sets the visibility state of the main menu popup
		if self.show_main_menu == false { self.show_main_menu = true; }
		else { self.show_main_menu = false; }
	}
}

// EOF