// planq.rs
// Provides the handling and abstractions for the player's PLANQ

use ratatui::backend::Backend;
use ratatui::Frame;
use ratatui::buffer::*;
use ratatui::layout::*;
use ratatui::layout::Rect;
use ratatui::widgets::*;
use ratatui::style::*;
use ratatui::text::Spans;
use bevy::prelude::*;
use crate::app::messagelog::Message;
use crate::components::Position;

/// Defines the Planq component within Bevy
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Planq { }

/// Defines the Planq status widget for ratatui: provides outputs directly from the Planq
/// as opposed to the CameraView, inventory display, &c, which use other Widgets
pub struct PlanqStatus<'a> {
	data: Vec<String>,
	block: Option<Block<'a>>,
	style: Style,
	align: Alignment,
}
impl<'a> PlanqStatus<'a> {
	pub fn new(new_data: &'a Vec<String>) -> PlanqStatus<'a> {
		PlanqStatus {
			data: new_data.to_vec(),
			block: None,
			style: Style::default(),
			align: Alignment::Left,
		}
	}
	pub fn block(mut self, block: Block<'a>) -> PlanqStatus<'a> {
		self.block = Some(block);
		self
	}
	pub fn style(mut self, style: Style) -> PlanqStatus<'a> {
		self.style = style;
		self
	}
	pub fn alignment(mut self, align: Alignment) -> PlanqStatus<'a> {
		self.align = align;
		self
	}
}
impl<'a> Widget for PlanqStatus<'a> {
	fn render(mut self, area: Rect, buf: &mut Buffer) {
		// Draw the border, if it exists
		let area = match self.block.take() {
			Some(b) => {
				let inner_area = b.inner(area);
				b.render(area, buf);
				inner_area
			}
			None => area,
		};
		// area now contains the remaining space to draw the PLANQ
		// anything wider than this is going to get truncated!
		let _max_width = area.right() - area.left();
		// The top and bottom panes are 'fixed' size, while the middle pane is expandable
		// TODO: The middle pane should be 'smart', and can count how many slots it has available
		//       for the player to load things into
		let textstyle = Style::default().fg(Color::White);
		// put the contents of self.data on the screen
		let mut y_index = area.top();
		for line in self.data {
			buf.set_string(area.left(), y_index, line, textstyle);
			y_index += 1;
		}
	}
}
/*
 * 

/// Defines the PLANQ operating terminal
pub struct PlanqTerm<'a> {
	stdout: Vec<Spans<'a>>,
	block: Option<Block<'a>>,
	style: Style,
	align: Alignment,
}
impl<'a> PlanqTerm<'a> {
	pub fn new(new_data: &'a Vec<String>) -> PlanqTerm<'a> {
		PlanqTerm {
			stdout: Vec::new(),
			block: None,
			style: Style::default(),
			align: Alignment::Left,
		}
	}
	pub fn block(mut self, block: Block<'a>) -> PlanqTerm<'a> {
		self.block = Some(block);
		self
	}
	pub fn style(mut self, style: Style) -> PlanqTerm<'a> {
		self.style = style;
		self
	}
	pub fn alignment(mut self, align: Alignment) -> PlanqTerm<'a> {
		self.align = align;
		self
	}
}
*/
/*
impl<'a> Widget for PlanqTerm<'a> {
	fn render(mut self, area: Rect, buf: &mut Buffer) {
		// Draw the border, if it exists
		let area = match self.block.take() {
			Some(b) => {
				let inner_area = b.inner(area);
				b.render(area, buf);
				inner_area
			}
			None => area,
		};
		// area now contains the remaining space to draw the PLANQ
		// anything wider than this is going to get truncated!
		let _max_width = area.right() - area.left();
		// The top and bottom panes are 'fixed' size, while the middle pane is expandable
		// TODO: The middle pane should be 'smart', and can count how many slots it has available
		//       for the player to load things into
		let textstyle = Style::default().fg(Color::White);
		// put the contents of self.data on the screen
		/*
		let mut y_index = area.top();
		for line in self.data {
			buf.set_string(area.left(), y_index, line, textstyle);
			y_index += 1;
		}
		*/
	}
}
*/
/*

/// Handles the output display for the Planq's main 'monitor'
pub struct PlanqOutput<'a> {
	stdout: Vec<String>,
	block: Option<Block<'a>>,
	style: Style,
	align: Alignment,
}
impl<'a> PlanqOutput<'a> {
	pub fn block(mut self, block: Block<'a>) -> PlanqOutput<'a> {
		self.block = Some(block);
		self
	}
	pub fn style(mut self, style: Style) -> PlanqOutput<'a> {
		self.style = style;
		self
	}
	pub fn alignment(mut self, align: Alignment) -> PlanqOutput<'a> {
		self.align = align;
		self
	}
}
impl<'a> Widget for PlanqOutput<'a> {
	/// This only renders the CLI input bar
	fn render(mut self, area: Rect, buf: &mut Buffer) {
		// Expect the area to be roughly rectangular, as it will be one of the two output screens
		// that are rendered on the sidebar
		let area = match self.block.take() {
			Some(b) => {
				let inner_area = b.inner(area);
				b.render(area, buf);
				inner_area
			}
			None => area,
		};
		// area now contains the remaining space to draw the PLANQ
		// anything wider than this is going to get truncated!
		
	}
}

*/
/// Provides the logic for the human input interface, including the command parser
#[derive(Resource, FromReflect, Reflect, Clone, Debug, Eq, PartialEq, Default)]
#[reflect(Resource)]
pub struct PlanqCmd {
	// put its local vars first
}
impl PlanqCmd {
	pub fn new() -> PlanqCmd {
		PlanqCmd {

		}
	}
}

/// Defines the set of output modes for the PLANQ's dual output windows
#[derive(FromReflect, Reflect, Default, Eq, PartialEq, Clone, Debug)]
pub enum PlanqOutputMode {
	#[default]
	Idle,
	InventoryChooser,
	Terminal,
	Settings,
}
/// Defines the Planq settings/controls (interface bwn my GameEngine class & Bevy)
#[derive(Resource, FromReflect, Reflect, Eq, PartialEq, Clone, Debug, Default)]
#[reflect(Resource)]
pub struct PlanqData {
	pub power_is_on: bool, // true if the planq has been turned on
	pub is_carried: bool, // true if the planq is in the player's inventory
	pub action_mode: PlanqActionMode, // Provides player action context for disambiguation
	pub output_1_enabled: bool,
	pub out1_mode: PlanqOutputMode,
	pub output_2_enabled: bool,
	pub out2_mode: PlanqOutputMode,
	pub show_inventory: bool,
	pub inventory_list: Vec<Entity>,
	pub player_loc: Position,
	pub show_cli_input: bool,
	pub cmd: PlanqCmd,
	pub stdout: Vec<Message>,
}
impl PlanqData {
	pub fn new() -> PlanqData {
		PlanqData {
			power_is_on: false,
			is_carried: false, // TODO: set this to detect actual carried status
			action_mode: PlanqActionMode::Default,
			output_1_enabled: false,
			out1_mode: PlanqOutputMode::Idle, // TODO: implement a 'screensaver' animation
			output_2_enabled: false,
			out2_mode: PlanqOutputMode::Terminal, // DEBUG: temp setting for testing
			show_inventory: false,
			inventory_list: Vec::new(),
			player_loc: Position::default(),
			show_cli_input: false,
			cmd: PlanqCmd::new(),
			stdout: Vec::new(),
		}
	}
	/// Provides the remaining amount of power in the PLANQ
	pub fn voltage() -> i32 {
		// TODO: implement battery storage/swapping
		return 100
	}
	pub fn inventory_toggle(&mut self) {
		if self.show_inventory == false { self.show_inventory = true; }
		else { self.show_inventory = false; }
	}
	/// Renders the status bars of the PLANQ
	pub fn render_status_bars<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		let mut planq_text = vec!["test string".to_string()]; // DEBUG:
		planq_text.push(format!("*D* x: {}, y: {}, z: {}",
		                        self.player_loc.x, self.player_loc.y, self.player_loc.z)); // DEBUG:
		frame.render_widget(
			PlanqStatus::new(&planq_text)
			.block(Block::default()
					.title("PLANQOS v29.3/rev30161124")
					.title_alignment(Alignment::Center)
					.borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
					.border_type(BorderType::Thick)
					.border_style(Style::default().fg(Color::White)),
			),
			area,
		);
	}
	/// Provides the contents of the PLANQ's stdout as a set of formatted Spans for ratatui
	pub fn get_stdout_as_spans(&self) -> Vec<Spans> {
		let mut output: Vec<Spans> = Vec::new();
		if self.stdout.is_empty() { return output; }
		for msg in self.stdout.iter() {
			output.push(msg.text.clone().into());
		}
		output
	}
	/// Renders the first (upper) PLANQ output window
	pub fn render_planq_stdout_1<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		// Switch based on the planq's output mode for this screen
		match self.out1_mode {
			PlanqOutputMode::Idle => { self.render_idle_mode(frame, area); }
			PlanqOutputMode::InventoryChooser => { self.render_item_chooser(frame, area); }
			PlanqOutputMode::Terminal => { self.render_terminal_output(frame, area); }
			PlanqOutputMode::Settings => { self.render_settings_menu(frame, area); }
		}
	}
	/// Renders the second (lower) PLANQ output window
	pub fn render_planq_stdout_2<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		// Switch based on the planq's output mode for this screen
		match self.out2_mode {
			PlanqOutputMode::Idle => { self.render_idle_mode(frame, area); }
			PlanqOutputMode::InventoryChooser => { self.render_item_chooser(frame, area); }
			PlanqOutputMode::Terminal => { self.render_terminal_output(frame, area); }
			PlanqOutputMode::Settings => { self.render_settings_menu(frame, area); }
		}
	}
	/*
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
	*/
	/*
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
	*/
	fn render_idle_mode<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		frame.render_widget(
			Paragraph::new("\n\n  (idling)")
			.block(Block::default()
			       .borders(Borders::ALL)
			       .border_style(Style::default().fg(Color::Green)),
			),
			area,
		);
	}
	fn render_item_chooser<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		frame.render_widget(
			Block::default(),
			area,
		);
	}
	fn render_settings_menu<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		frame.render_widget(
			Block::default(),
			area,
		);
	}
	fn render_terminal_output<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		frame.render_widget(
			Paragraph::new(self.get_stdout_as_spans())
			.block(Block::default())
			.style(Style::default())
			.wrap(Wrap { trim: true }),
			area,
		);
	}
}
/// Provides context for certain actions (inventory use/drop, &c) that take secondary/JIT inputs
#[derive(Resource, FromReflect, Reflect, Default, Clone, Debug, Eq, PartialEq)]
#[reflect(Resource)]
pub enum PlanqActionMode {
	#[default]
	Default,
	DropItem,
	UseItem,
	CliInput,
}

// EOF
