// planq.rs
// Provides the handling and abstractions for the player's PLANQ

use std::fmt;
use std::fmt::Display;
use std::collections::{HashMap, VecDeque};
use ratatui::backend::Backend;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::*;
use ratatui::style::*;
use ratatui::text::Spans;
use bevy::prelude::*;
use bevy::utils::Duration;
use crate::app::messagelog::Message;
use crate::components::Position;
use crate::app::event::PlanqEvent;
use crate::app::PlanqCmd::*;
use strum_macros::EnumIter;

use tui_textarea::TextArea;

/// Defines the Planq settings & controls (interface bwn my GameEngine class & Bevy)
#[derive(Resource, FromReflect, Reflect, Eq, PartialEq, Clone, Debug, Default)]
#[reflect(Resource)]
pub struct PlanqData {
	pub power_is_on: bool, // true if the planq has been turned on
	pub boot_stage: u32,
	pub is_carried: bool, // true if the planq is in the player's inventory
	pub cpu_mode: PlanqCPUMode,
	pub action_mode: PlanqActionMode, // Provides player action context for disambiguation
	pub show_terminal: bool,
	pub show_inventory: bool,
	pub inventory_list: Vec<Entity>,
	pub player_loc: Position, // DEBUG: current player location
	pub show_cli_input: bool,
	pub stdout: Vec<Message>, // Contains the PLANQ's message backlog
	pub proc_table: Vec<Entity>, // The list of PlanqProcesses running in the Planq
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
			show_terminal: false,
			show_inventory: false,
			inventory_list: Vec::new(),
			player_loc: Position::default(),
			show_cli_input: false,
			stdout: Vec::new(),
			proc_table: Vec::new(),
		}
	}
	pub fn inventory_toggle(&mut self) {
		if !self.show_inventory { self.show_inventory = true; }
		else { self.show_inventory = false; }
	}
	/// Renders the CLI input box
	pub fn render_cli<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect, stdin: &mut PlanqInput) {
		stdin.input.set_block(
			Block::default()
			.borders(Borders::LEFT | Borders::RIGHT)
			.border_type(BorderType::Plain)
		);
		frame.render_widget(stdin.input.widget(), area);
	}
	/// Renders the whole terminal window, including the backlog, leaving room for the CLI
	pub fn render_terminal<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		/*
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
			*/
		let stdout = self.get_stdout_as_spans();
		let start_offset = (stdout.len() as i32) - area.height as i32 + 2;
		let mut start: usize = 0;
		if start_offset > 0 { start = start_offset as usize; }
		let backscroll = stdout[start..].to_vec();
		frame.render_widget(
			Paragraph::new(backscroll)
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
	/*
	/// Executes a command on the PLANQ, generally from the CLI
	pub fn exec(&mut self, cmd: PlanqCmd) -> bool {
		match cmd {
			PlanqCmd::Error(msg) => {
				self.stdout.push(Message::new(0, 0, "planq".to_string(), "& ERROR:".to_string()));
				self.stdout.push(Message::new(0, 0, "planq".to_string(), format!("& {}", msg)));
			}
			PlanqCmd::Help => { /* list all the PLANQ commands */ }
			PlanqCmd::Shutdown => { /* trigger a shutdown */ }
			PlanqCmd::Reboot => { /* execute a reboot */ }
			PlanqCmd::Connect(target) => { /* run the planq.connect subroutine */ }
			PlanqCmd::Disconnect => { /* run the planq.disconnect subroutine */ }
			_ => { /* NoOperation */ }
		}
		false
	}
	*/
}

/// Handles the PLANQ's status bars, their settings, their inputs, &c
#[derive(FromReflect, Reflect, Resource, Eq, PartialEq, Clone, Debug)]
#[reflect(Resource)]
pub struct PlanqMonitor {
	pub status_bars: Vec<String>, // The list of active statusbar modules
	pub raw_data: HashMap<String, PlanqDataType>, // Contains the live monitoring data
}
impl PlanqMonitor {
	// Builders
	pub fn new() -> PlanqMonitor {
		PlanqMonitor::default()
	}
	pub fn watch(mut self, source: String) -> Self {
		self.status_bars.push(source);
		self
	}
	// General
	/// Removes the specified source from the list of status_bars, thus removing it from the PLANQ
	/// Returns true if the source was successfully removed
	pub fn remove(mut self, source: String) -> bool {
		if let Some(posn) = self.status_bars.iter().position(|x| x == source.as_str()) {
			self.status_bars.remove(posn);
			return true;
		}
		false
	}
	/// Describes how the PLANQ's monitor will render to the screen
	/// Note that the area parameter should be just the sidebar area, not including the terminal
	pub fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>, mut area: Rect) {
		// TODO: Sparkline's height can be constrained by its area.height, need to check Gauge widget
		area.height = 1;
		let default_block = Block::default().borders(Borders::LEFT | Borders::RIGHT).border_type(BorderType::Plain)
			.border_style(Style::default().fg(Color::Gray));
		// NOTE: Previously tried to implement this logic using another fxn to do dynamic dispatch
		//       Unfortunately, in Rust, trait objects cannot be passed as params or instantiated locally
		//       They can be Boxed, but because the Widget type does not impl the Sized trait,
		//       using a Box to handle the dispatch fails when Rust tries to calculate types at compilation
		//       Thus: all who might modify this logic, BEWARE
		// METHOD
		// For each data_source in the status_bars list,
		// 1: try to retrieve the data associated with the source from the data_source dictionary
		// 2: if successful, match the retrieved data with a PlanqDataType
		// 3: for that PDT, check if the data source is a special case, and if so, use that logic for display
		// 4: else, just display the data using a generic pattern for that PDT
		for source in &self.status_bars {
			// TODO: These will need a revisit for formatting, sanity, &c
			if let Some(source_type) = self.raw_data.get(source) {
				match source_type {
					PlanqDataType::Text(text) => {
						// TODO: these prefixes could probably get promoted into a dict or something faster/precompiled
						let prefix = match source.as_str() {
							"planq_mode" => { "MODE: ".to_string() }
							"player_location" => { "LOCN: ".to_string() }
							"current_time" => { "TIME: ".to_string() }
							_ => { "".to_string() }
						};
						let remainder = area.width as usize - prefix.len() - 2;
						let line = PlanqMonitor::right_align(text.clone(), remainder);
						let output = prefix + &line;
						frame.render_widget(Paragraph::new(output).block(default_block.clone()), area);
					}
					PlanqDataType::Integer(val) => {
						frame.render_widget(Paragraph::new(val.to_string())
						                    .block(default_block.clone()), area);
					}
					PlanqDataType::Percent(pct) => {
						if source == "planq_battery" {
							let prefix = "BATT: ".to_string();
							let remainder = area.width as usize - prefix.len() - 2;
							let line = PlanqMonitor::right_align(pct.to_string() + "%", remainder);
							let output = prefix + &line;
							frame.render_widget(Gauge::default().percent(*pct as u16).label(format!("{:width$}", output, width = area.width as usize))
							                    .gauge_style(Style::default().fg(Color::White).bg(Color::Black))
							                    .block(default_block.clone()), area)
						} else {
							frame.render_widget(Gauge::default().percent(*pct as u16)
							                    .gauge_style(Style::default().fg(Color::White).bg(Color::Black))
							                    .block(default_block.clone()), area)
						}
					}
					PlanqDataType::Decimal { numer, denom } => {
						let quotient: f64 = *numer as f64 / *denom as f64;
						frame.render_widget(LineGauge::default().ratio(quotient)
						                    .gauge_style(Style::default().fg(Color::White).bg(Color::Blue))
						                    .block(default_block.clone()), area);
					}
					PlanqDataType::Series(data) => {
						// NOTE: Sparkline's default for max() will be highest value in series if not specified
						let series = Vec::from(data.clone()); // Convert it to a Vec from a VecDeque
						frame.render_widget(Sparkline::default().data(&series)
						                    .block(default_block.clone()), area);
					}
					_ => { continue; } // Covers the Null type
				};
				area.y += 1;
			} else {
				continue;
			}
		}
	}
	/// Prepends whitespace to the given string until it is of the given width, for right-aligning PLANQ text
	/// Can be used to build empty lines by giving an empty string to prepend to
	// TODO: perhaps write a "hard_right_align" that truncates if the string is too long?
	// NOTE: Rust technically allows padding with an arbitrary char, but the std::fmt macros do not provide any way
	//         to change this at runtime, since it has to be included as part of the format! macro
	//       If string padding with arbitrary chars is desired, must either:
	//         consistently use the same char every time,
	//         or use an external crate that provides the syntax
	fn right_align(input: String, width: usize) -> String {
		if input.len() >= width { return input; }
		format!("{:>str_width$}", input, str_width = width)
	}
}
impl Default for PlanqMonitor {
	fn default() -> PlanqMonitor {
		PlanqMonitor {
			status_bars: vec!["planq_battery".to_string(), "planq_mode".to_string(), "current_time".to_string(), ],
			raw_data: HashMap::from([("current_time".to_string(), PlanqDataType::Text("Initializing...".to_string())),
				                       ("planq_battery".to_string(), PlanqDataType::Percent(0)),
				                       ("planq_mode".to_string(), PlanqDataType::Text("Initializing...".to_string()))
			]),
		}
	}
}

/// Defines the set of possible data types that a PLANQ's data source might provide
#[derive(FromReflect, Reflect, Eq, PartialEq, Clone, Debug, Default)]
pub enum PlanqDataType {
	#[default]
	Null,
	Text(String), // Ideally this should be a Span or some other ratatui-compat type instead
	Integer(i32),
	Percent(u32),
	Decimal{numer: i32, denom: i32}, // Floating point numbers don't impl Eq, only PartialEq, so we have to use this pair of ints as a fractional representation instead
	Series(VecDeque<u64>),
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
impl Display for PlanqCPUMode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let output = match *self {
			PlanqCPUMode::Idle => { "IDLE" }
			PlanqCPUMode::Error(_) => { "ERROR" }
			PlanqCPUMode::Startup => { "STARTUP" }
			PlanqCPUMode::Shutdown => { "SHUTDOWN" }
			PlanqCPUMode::Working => { "WORKING" }
			PlanqCPUMode::Offline => { "OFFLINE" }
		};
		write!(f, "{}", output)
	}
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
/// Defines the full set of commands that can actually be executed on the PLANQ
#[derive(FromReflect, Reflect, PartialEq, Eq, Clone, Debug, Default, EnumIter)]
pub enum PlanqCmd {
	#[default]
	NoOperation,
	Error(String),
	Help,
	Shutdown,
	Reboot,
	Connect(String),
	Disconnect
}
impl fmt::Display for PlanqCmd {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		//write!(f, "{}", self.field)
		match *self {
			NoOperation => { write!(f, "(NoOperation)") }
			Error(_) => { write!(f, "(Error)") }
			Help => { write!(f, "help") }
			Shutdown => { write!(f, "shutdown") }
			Reboot => { write!(f, "reboot") }
			Connect(_) => { write!(f, "connect") }
			Disconnect => { write!(f, "disconnect") }
		}
	}
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
		PlanqProcess::default()
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
/// Provides a means for setting regular intervals for the PLANQ's monitoring, so that we are not
/// forced to provide updates at the framerate (and possibly cause flickering, &c)
/// If no duration is specified, the DataSample source will always be updated
#[derive(FromReflect, Reflect, Component, Clone, Default)]
#[reflect(Component)]
pub struct DataSampleTimer {
	pub timer: Timer,
	pub source: String,
}
impl DataSampleTimer {
	pub fn new() -> DataSampleTimer {
		DataSampleTimer::default()
	}
	pub fn duration(mut self, duration: u64) -> Self {
		self.timer = Timer::new(Duration::from_secs(duration), TimerMode::Repeating);
		self
	}
	pub fn source(mut self, source: String) -> Self {
		self.source = source;
		self
	}
}
/// Defines the Planq 'tag' component within Bevy; used only as a lightweight marker for the PLANQ entity
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Planq { }

// EOF
