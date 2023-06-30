// planq.rs
// Provides the handling and abstractions for the player's PLANQ

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
}

/// Handles the PLANQ's status bars, their settings, their inputs, &c
#[derive(FromReflect, Reflect, Resource, Default, Eq, PartialEq, Clone, Debug)]
#[reflect(Resource)]
pub struct PlanqMonitor {
	pub status_bars: Vec<String>, // The list of active statusbar modules
	pub raw_data: HashMap<String, PlanqDataSource>, // Contains the live monitoring data
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
		for source in &self.status_bars {
			// NOTE: Previously tried to implement this logic using another fxn to do dynamic dispatch
			// Unfortunately, in Rust, trait objects cannot be passed as params or instantiated locally
			// They can be Boxed, but because the Widget type does not impl the Sized trait,
			// using a Box to handle the dispatch fails when Rust tries to calculate types at compilation
			// Therefore, if you know what's good for you, don't try to refactor this pattern...
			// TODO: These will need a revisit for formatting, sanity, &c
			if let Some(source_type) = self.raw_data.get(source) {
				match source_type {
					PlanqDataSource::Text(text) => {
						//eprintln!("raw_data area height: {}", area.height);
						frame.render_widget(Paragraph::new(text.clone())
						                    .block(default_block.clone()), area);
					}
					PlanqDataSource::Integer(val) => {
						frame.render_widget(Paragraph::new(val.to_string())
						                    .block(default_block.clone()), area);
					}
					PlanqDataSource::Percent(pct) => {
						//eprintln!("gauge area height: {}", area.height);
						frame.render_widget(Gauge::default().percent(*pct as u16)
						                    .gauge_style(Style::default().fg(Color::Red).bg(Color::Green))
						                    .block(default_block.clone()), area)
					}
					PlanqDataSource::Decimal { numer, denom } => {
						//eprintln!("linegauge area height: {}", area.height);
						let quotient: f64 = *numer as f64 / *denom as f64;
						frame.render_widget(LineGauge::default().ratio(quotient)
						                    .gauge_style(Style::default().fg(Color::White).bg(Color::Blue))
						                    .block(default_block.clone()), area);
					}
					PlanqDataSource::Series(data) => {
						//eprintln!("sparkline area height: {}", area.height);
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
}
/// Defines the set of possible data types that a PLANQ's data source might provide
#[derive(FromReflect, Reflect, Eq, PartialEq, Clone, Debug, Default)]
pub enum PlanqDataSource {
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
