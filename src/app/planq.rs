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
use bevy::utils::Duration;
use crate::app::messagelog::Message;
use crate::components::Position;
use crate::app::event::PlanqEvent;

use tui_textarea::TextArea;

/// Defines the Planq 'tag' component within Bevy
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Planq { }

/// Determines the type of status bar that will be displayed in the Planq
#[derive(FromReflect, Reflect, Eq, PartialEq, Clone, Debug, Default)]
pub enum PlanqStatusBarType {
	#[default]
	Null,         // Displays nothing, provided for compatibility
	RawData(String),    // Paragraph (ie display data directly)
	Gauge(u16),      // Gauge,LineGauge
//	Sparkline,  // Sparkline
//	Chart,      // Chart
//	Graph,      // BarChart
//	Canvas,     // Canvas
//	Table,      // Table
}
#[derive(FromReflect, Reflect, Resource, Default, Eq, PartialEq, Clone, Debug)]
#[reflect(Resource)]
pub struct PlanqBar {
	pub btype:  PlanqStatusBarType,
	// -> START HERE: implement the first 'small' types of status bar, see the notes
	// will probably want some kind of trait object as each status bar type takes a
	// different kind of data as input: the 'render' call that receives the data should
	// be generalized across the various specific types
}

/// Defines the Planq status widget for ratatui: provides outputs directly from the Planq
/// as opposed to the CameraView, inventory display, &c, which use other Widgets
pub struct PlanqStatus<'a> {
	data: Vec<String>,
	block: Option<Block<'a>>,
	style: Style,
	align: Alignment,
}
impl<'a> PlanqStatus<'a> {
	pub fn new(new_data: &'a [String]) -> PlanqStatus<'a> {
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
/// Defines the CLI input system and its logic
/// Note that tui-textarea is a part of the ratatui ecosystem, and therefore
/// is ineligible by definition for addition to the Bevy ecosystem
#[derive(Default, Clone)]
pub struct PlanqInput<'a> {
	//pub input: Input, // This cannot be added to anything with Reflect, nor can it have Reflect implemented for it because it is external
	pub input: TextArea<'a>,
	pub history: Vec<String>,
}
impl PlanqInput<'_> {
	pub fn new() -> PlanqInput<'static> {
		PlanqInput {
			input: TextArea::default(),
			history: Vec::new(),
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
/// Defines the set of operating modes in the PLANQ's firmware
#[derive(FromReflect, Reflect, Default, Eq, PartialEq, Clone, Debug)]
pub enum PlanqCPUMode {
	#[default]
	Idle,
	Error(u32),
	Startup,
	Shutdown,
	Working,
	Offline,
}
/// Provides context for certain actions (inventory use/drop, &c) that take secondary inputs
#[derive(FromReflect, Reflect, Default, Clone, Debug, Eq, PartialEq)]
pub enum PlanqActionMode {
	#[default]
	Default,
	DropItem,
	UseItem,
	CliInput,
}
/// Provides the Bevy-backed tools for doing things on the PLANQ involving time intervals
/// That is, this represents a 'process' or task within the PLANQ that needs processing time to complete
#[derive(FromReflect, Reflect, Component, Clone, Default)]
#[reflect(Component)]
pub struct PlanqProcess {
	pub timer: Timer,
	pub outcome: PlanqEvent,
}
impl PlanqProcess {
	pub fn new() -> PlanqProcess {
		PlanqProcess {
			timer: Timer::default(),
			outcome: PlanqEvent::default()
		}
	}
	pub fn time(mut self, duration: u64) -> PlanqProcess {
		self.timer = Timer::new(Duration::from_secs(duration), TimerMode::Once);
		self
	}
	pub fn event(mut self, new_event: PlanqEvent) -> PlanqProcess {
		self.outcome = new_event;
		self
	}
}
/// Defines the Planq settings & controls (interface bwn my GameEngine class & Bevy)
#[derive(Resource, FromReflect, Reflect, Eq, PartialEq, Clone, Debug, Default)]
#[reflect(Resource)]
pub struct PlanqData {
	pub power_is_on: bool, // true if the planq has been turned on
	pub boot_stage: u32,
	pub is_carried: bool, // true if the planq is in the player's inventory
	pub cpu_mode: PlanqCPUMode,
	pub action_mode: PlanqActionMode, // Provides player action context for disambiguation
	//pub output_1_enabled: bool,
	//pub out1_mode: PlanqOutputMode,
	//pub output_2_enabled: bool,
	//pub out2_mode: PlanqOutputMode,
	pub show_terminal: bool,
	pub terminal_mode: PlanqOutputMode,
	pub show_inventory: bool,
	pub inventory_list: Vec<Entity>,
	pub player_loc: Position,
	pub show_cli_input: bool,
	pub stdout: Vec<Message>, // Contains the PLANQ's message backlog
	pub proc_table: Vec<Entity>, // The list of PlanqProcesses running in the Planq
	pub status_bars: Vec<PlanqBar>, // The list of active statusbar modules
	// trying to maintain a vec of PlanqStatus objects would be tough because of lifetimes
	// not sure yet what is required to help this work without storing a PlanqStatus directly
}
impl PlanqData {
	pub fn new() -> PlanqData {
		PlanqData {
			power_is_on: false,
			boot_stage: 0,
			is_carried: false,
			cpu_mode: PlanqCPUMode::Offline,
			action_mode: PlanqActionMode::Default,
			//output_1_enabled: false,
			//out1_mode: PlanqOutputMode::Terminal,
			//output_2_enabled: false,
			//out2_mode: PlanqOutputMode::Idle,
			show_terminal: false,
			terminal_mode: PlanqOutputMode::Terminal,
			show_inventory: false,
			inventory_list: Vec::new(),
			player_loc: Position::default(),
			show_cli_input: false,
			stdout: Vec::new(),
			proc_table: Vec::new(),
			status_bars: Vec::new(),
		}
	}
	pub fn inventory_toggle(&mut self) {
		if !self.show_inventory { self.show_inventory = true; }
		else { self.show_inventory = false; }
	}
	/// Renders the status bars of the PLANQ
	pub fn render_status_bars<B: Backend>(&mut self, frame: &mut Frame<'_, B>, mut area: Rect) {
		//let mut planq_text = vec!["test string".to_string()]; // DEBUG:
		//planq_text.push(format!("*D* x: {}, y: {}, z: {}",
		//                        self.player_loc.x, self.player_loc.y, self.player_loc.z)); // DEBUG:
		//planq_text.push("123456789_123456789_123456789_".to_string()); // DEBUG: ruler
		//frame.render_widget(
		//	PlanqStatus::new(&planq_text)
		//	.block(Block::default()
		//			.title("PLANQOS v29.3/rev30161124")
		//			.title_alignment(Alignment::Right)
		//			.borders(Borders::ALL)
		//			.border_type(BorderType::Plain)
		//			.border_style(Style::default().fg(Color::White)),
		//	),
		//	area,
		//);
		// ***
		let sbar_block = Block::default().borders(Borders::LEFT | Borders::RIGHT)
			.border_type(BorderType::Plain)
			.border_style(Style::default().fg(Color::Gray));
		for sbar in &self.status_bars {
			match &sbar.btype {
				PlanqStatusBarType::RawData(input) => {
					frame.render_widget(Paragraph::new(input.clone()).block(sbar_block.clone()), area);
				}
				PlanqStatusBarType::Gauge(input) => {
					frame.render_widget(Gauge::default().percent(*input).block(sbar_block.clone()), area);
				}
				_ => { }
			}
			area.y += 1;
		}
	}
	/// Renders the CLI input box
	pub fn render_cli<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect, stdin: &mut PlanqInput) {
		//let mut cli = TextArea::default();
		//cli.set_block(
		stdin.input.set_block(
			Block::default()
			.borders(Borders::LEFT | Borders::RIGHT)
			.border_type(BorderType::Plain)
		);
		//frame.render_widget(cli.widget(), area);
		frame.render_widget(stdin.input.widget(), area);
	}
	/// Renders the whole terminal window, including the backlog, leaving room for the CLI
	pub fn render_terminal<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		frame.render_widget(
			Paragraph::new("self.stdout")
			.block(Block::default()
			       .borders(Borders::ALL)
			       .border_type(BorderType::Plain)
			       .border_style(Style::default().fg(Color::Blue)),
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
}

// EOF
