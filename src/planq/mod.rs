// planq/mod.rs
// Provides all of the logic and handling for the player's PLANQ

#![allow(clippy::too_many_arguments)]

//  ###: EXTERNAL LIBRARIES
use bevy::{
	prelude::*,
	ecs::query::*,
	//utils::*,
	utils::Duration,
};
use ratatui::prelude::*;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui::widgets::*;
use strum_macros::EnumIter;

//  ###: INTERNAL LIBRARIES
use crate::{
	components::*,
	engine::event::*,
	engine::messagelog::*,
	planq::{
		tui::*,
		PlanqEventType::*,
	},
};
pub mod monitor;
pub mod tui;

//  ###: COMPLEX TYPES


//  ###: BEVY SYSTEMS
/// Allows us to run PLANQ updates and methods in their own thread, just like a real computer~
pub fn planq_update_system(mut commands: Commands,
	                         mut ereader:  EventReader<GameEvent>,
	                         mut preader:  EventReader<PlanqEvent>,
	                         mut msglog:   ResMut<MessageLog>,
	                         time:         Res<Time>,
	                         mut planq:    ResMut<PlanqData>, // contains the PLANQ's settings and data storage
	                         p_query:      Query<(Entity, &Body), With<Player>>, // provides interface to player data
	                         mut q_query:  Query<(Entity, &Device, &Portable), With<Planq>>, // contains the PLANQ's component data
	                         mut t_query:  Query<(Entity, &mut PlanqProcess)>, // contains the set of all PlanqTimers
) {
	if p_query.is_empty() { return; }
	if q_query.is_empty() { return; }
	let (p_enty, _body) = if let Ok(value) = p_query.get_single() { value } else { return };
	let (q_enty, q_device, q_portable) = if let Ok(value) = q_query.get_single_mut() { value } else { return };
	// Handle any new GameEvents we're interested in
	if !ereader.is_empty() {
		for event in ereader.read() {
			let atype;
			if let GameEventType::PlayerAction(action) = event.etype {
				atype = action;
			} else {
				continue;
			}
			match atype {
				// Player interaction events that need to be monitored
				ActionType::MoveItem => { // The player (g)ot the PLANQ from somewhere external
					if let Some(econtext) = event.context.as_ref() {
						planq.is_carried = econtext.subject == p_enty && econtext.object == q_enty;
					}
				}
				ActionType::DropItem => { // The player (d)ropped the PLANQ
					if let Some(econtext) = event.context.as_ref() {
						if econtext.object == q_enty { planq.is_carried = false; }
					}
				}
				ActionType::UseItem => { // The player (a)pplied the PLANQ
					if let Some(econtext) = event.context.as_ref() {
						if econtext.subject == p_enty
						&& econtext.object == q_enty {
							// Note that the Operable system already handles the ItemUse action for the
							// PLANQ: it allows the player to operate the power switch
							// This seems likely to change in the future to allow some better service
							// commands, like battery swaps or peripheral attachment
							msglog.tell_player("There is a faint 'click' as you press the PLANQ's power button.");
						}
					}
				}
				_ => { }
			}
		}
	}
	// Handle all new PlanqEvents
	if !preader.is_empty() {
		for event in preader.read() {
			match event.etype {
				// PLANQ system commands
				PlanqEventType::NullEvent      => { /* do nothing */ }
				PlanqEventType::Startup        => { planq.cpu_mode = PlanqCPUMode::Startup; } // covers the entire boot stage
				PlanqEventType::BootStage(lvl) => { planq.boot_stage = lvl; }
				PlanqEventType::Shutdown       => { planq.cpu_mode = PlanqCPUMode::Shutdown; }
				PlanqEventType::Reboot         => { todo!(">>> planq.rs:planq_update_system(), l95 - implement PlanqEventType::Reboot"); /* TODO: do a Shutdown, then a Startup */ }
				PlanqEventType::GoIdle         => { planq.idle_mode(&mut msglog); }
				PlanqEventType::CliOpen => {
					planq.show_cli_input = true;
					planq.action_mode = PlanqActionMode::CliInput;
				}
				PlanqEventType::CliClose => {
					// FIXME: need to clear the CLI's input buffer! might need to do this at the time of key input?
					planq.show_cli_input = false;
					planq.action_mode = PlanqActionMode::Default; // FIXME: this might be a bad choice
				}
				PlanqEventType::AccessLink => {
					// The player has connected the PLANQ's access jack to an AccessPort (PlanqConnect has fired)
					// but has not yet executed "connect" on the PLANQ itself (PlanqCmd::Connect(target))
					// planq.jack_cnxn needs to contain the Entity ID of the target
					// - Set up whatever backend linkage is needed
					// - Get the status output of the target
					// - Display that status output and switch back to Idle
					// OUTPUT:789_123456789_123456789_
					// "P: Connected: $ENTY"
					// "E: Status: $E_STATUS"
					// "P: (idle)"
					todo!(">>> planq.rs:planq_update_system(), l125 - implement PlanqEventType::AccessLink");
				}
				PlanqEventType::AccessUnlink => {
					// The player has disconnected their PLANQ from the AccessPort
					// - If PlanqCmd::Disconnect() was not run prior, may wish to capture that and cause errors
					// - stop any running processes/jobs
					// - stop/clean up any leftover bits
					// - return to the main PLANQ input state (Working/Idle)
					// OUTPUT:789_123456789_123456789_
					// "P: Connection closed"
					// "P: (idle)"
					todo!(">>> planq.rs:planq_update_system(), l125 - implement PlanqEventType::AccessUnlink");
				}
			}
		}
	}
	// Update the PLANQData resources:
	// - Get the device hardware info
	if !planq.power_is_on && q_device.pw_switch {
		planq.power_is_on = q_device.pw_switch; // Update the power switch setting
		planq.show_terminal = true;
		planq.cpu_mode = PlanqCPUMode::Startup; // Begin booting the PLANQ's OS
	}
	if planq.power_is_on && !q_device.pw_switch {
		planq.power_is_on = q_device.pw_switch; // Update the power switch setting
		planq.cpu_mode = PlanqCPUMode::Shutdown; // Initiate a shutdown
	}
	// - Handle the Planq's CPU mode logic
	// CRASH CHECK:
	if planq.power_is_on // IF the PLANQ is powered on,
	&& planq.proc_table.is_empty() // BUT there are no running processes (!),
	&& (planq.cpu_mode == PlanqCPUMode::Working || planq.cpu_mode == PlanqCPUMode::Idle) { // BUT the PLANQ is supposed to be running (!!)
		planq.cpu_mode = PlanqCPUMode::Error(420); // Switch to an error mode
	}
	match planq.cpu_mode {
		PlanqCPUMode::Error(_) => { todo!(">>> planq.rs:planq_update_system(), l147 - implement Error state"); }
		PlanqCPUMode::Offline  => { /* do nothing */ }
		PlanqCPUMode::Startup  => {
			// do the boot process: send outputs, progress bars, the works
			// then kick over to PCM::Idle
			if !planq.proc_table.is_empty() {
				// if there are any running processes, check to see if they're done
				for id in planq.proc_table.clone() {
					if let Ok((_proc_enty, q_proc_data)) = t_query.get(id) {
						if q_proc_data.timer.just_finished() {
							match q_proc_data.outcome.etype {
								BootStage(lvl) => {
									planq.boot_stage = lvl;
								}
								PlanqEventType::GoIdle => { planq.idle_mode(&mut msglog); }
								_ => { }
							}
						}
					}
				}
			}
			// Get proc 0, aka the boot process
			let proc_ref = if !planq.proc_table.is_empty() {
				t_query.get_mut(planq.proc_table[0])
			} else {
				Err(QueryEntityError::NoSuchEntity(Entity::PLACEHOLDER))
			};
			match planq.boot_stage {
				0 => {
					if planq.proc_table.is_empty() {
						//debug!("¶ running boot stage {}", planq.boot_stage); // DEBUG: announce the current PLANQ boot stage
						msglog.boot_message(planq.boot_stage);
						// kick off boot stage 1
						planq.proc_table.push(commands.spawn(
								PlanqProcess::new()
								.time(3)
								.event(PlanqEvent::new(PlanqEventType::BootStage(1))))
							.id()
						);
					}
				}
				1 => {
					if let Ok((_enty, mut proc)) = proc_ref {
						if proc.timer.just_finished() {
							//debug!("¶ running boot stage {}", planq.boot_stage); // DEBUG: announce the current PLANQ boot stage
							msglog.boot_message(planq.boot_stage);
							// set its duration, if needed
							//proc.1.timer.set_duration(Duration::from_secs(5));
							// reset it
							proc.timer.reset(); // will be iterated on at next system run
							proc.outcome = PlanqEvent::new(PlanqEventType::BootStage(2));
						}
					}
				}
				2 => {
					if let Ok((_enty, mut proc)) = proc_ref {
						if proc.timer.just_finished() {
							//debug!("¶ running boot stage {}", planq.boot_stage); // DEBUG: announce the current PLANQ boot stage
							msglog.boot_message(planq.boot_stage);
							// set its duration, if needed
							//proc.1.timer.set_duration(Duration::from_secs(5));
							// reset it and start it
							proc.timer.reset(); // will be iterated on at next system run
							proc.outcome = PlanqEvent::new(PlanqEventType::BootStage(3));
						}
					}
				}
				3 => {
					if let Ok((_enty, mut proc)) = proc_ref {
						if proc.timer.just_finished() {
							//debug!("¶ running boot stage {}", planq.boot_stage); // DEBUG: announce the current PLANQ boot stage
							msglog.boot_message(planq.boot_stage);
							// set its duration, if needed
							//proc.1.timer.set_duration(Duration::from_secs(5));
							// reset it and start it
							proc.timer.reset(); // will be iterated on at next system run
							proc.outcome = PlanqEvent::new(PlanqEventType::BootStage(4));
						}
					}
				}
				4 => {
					if let Ok((_enty, mut proc)) = proc_ref {
						if proc.timer.just_finished() {
							//debug!("¶ running boot stage {}", planq.boot_stage); // DEBUG: announce the current PLANQ boot stage
							msglog.boot_message(planq.boot_stage);
							proc.outcome = PlanqEvent::new(PlanqEventType::NullEvent);
							planq.idle_mode(&mut msglog);
						}
					}
				}
				_ => { }
			}
		}
		PlanqCPUMode::Shutdown => {
			// Make sure the proc_table is clear
			// Set the CPU's mode
			// When finished, set the power_is_on AND planq_enty.2.pw_switch to false
			todo!(">>> planq.rs:planq_update_system(), l258 - implement PlanqCPUMode::Shutdown");
		}
		PlanqCPUMode::Idle     => {
			/*
			// IDLE GRAPHIC: Bouncing Box
			// Given a sequence of integers 0-9, produce a smoothly scaled integer 1-21:
			let smooth_input = (time.elapsed().as_secs() % 10) as f64;
			//let angle: f64 = 0.6282 * smooth_input - 1.571;
			//let output = (10.5 * angle.sin() + 10.5) as usize;
			let output = (4.4 * smooth_input - 23.0).abs() as usize;
			// Creates the new idle image by prepending with a variable number of spaces, so that the graphic 'moves'
			let idle_message = format!("{:width$}", "", width=output) + "-=[ ]=-";
			*/
			/*
			// IDLE GRAPHIC: Bizarre Data
			let sample = vec!['▖', '▗', '▘', '▝', '▀', '▄', '▌', '▐', '▚', '▞', '▙', '▛', '▜', '▟', '█'];
			// randomly pick chars from sample until we have a line of the correct width
			let mut idle_message = "".to_string();
			for _ in 0..30 {
				let choice = rng.usize(0..sample.len());
				idle_message.push(sample[choice]);
			}
			*/
			// Update the idle message if there's nothing waiting for processing
			if planq.proc_table.len() == 1 {
				//msglog.replace(idle_message, "planq".to_string(), 0, 0); // continue idling
			} else {
				planq.cpu_mode = PlanqCPUMode::Working;
			}
		}
		PlanqCPUMode::Working  => {
			// Display the outputs from the workloads
			// If all workloads are done, shift back to Idle mode
			if planq.proc_table.len() == 1 { planq.idle_mode(&mut msglog); }
		}
	}
	// - Iterate any active PlanqProcesses (these are NOT DataSampleTimers!)
	for (_enty, mut proc) in t_query.iter_mut() {
		if !proc.timer.finished() {
			proc.timer.tick(time.delta());
		}
	}
	// - Check for some edge cases and other things that we'd like to avoid
	if planq.is_carried && q_portable.carrier != p_enty { planq.is_carried = false; }
	if !planq.is_carried && q_portable.carrier == p_enty { planq.is_carried = true; }
}

/// BEVY: Defines the Planq settings/controls (interface bwn my GameEngine class & Bevy)
#[derive(Resource, Clone, Debug, PartialEq, Eq, Reflect)]
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
	pub player_loc: Position,
	pub show_cli_input: bool,
	pub stdout: Vec<Message>, // Local copy of the PLANQ's message backlog, as copied from the MessageLog "planq" channel
	pub proc_table: Vec<Entity>, // The list of PlanqProcesses running in the Planq
	pub jack_cnxn: Entity, // ID of the object that the PLANQ's access jack is connected to
}
impl Default for PlanqData {
	fn default() -> PlanqData {
		PlanqData {
			power_is_on: false, // true if the planq has been turned on
			boot_stage: 0,
			is_carried: false, // true if the planq is in the player's inventory
			cpu_mode: PlanqCPUMode::Offline,
			action_mode: PlanqActionMode::Default, // Provides player action context for disambiguation
			show_terminal: false,
			show_inventory: false,
			inventory_list: Vec::new(),
			player_loc: Position::default(), // player's current coordinates (TODO: replace with a room-based system)
			show_cli_input: false,
			stdout: Vec::new(), // Contains the PLANQ's message backlog
			proc_table: Vec::new(), // The list of PlanqProcesses running in the Planq
			jack_cnxn: Entity::PLACEHOLDER, // ID of the object that the PLANQ's access jack is connected to
		}
	}
}
impl PlanqData {
	pub fn new() -> PlanqData {
		PlanqData::default()
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
		frame.render_widget(stdin.input.widget(), area);
	}
	/// Renders the whole terminal window, including the backlog, leaving room for the CLI
	pub fn render_terminal<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		let stdout = self.get_stdout_as_lines();
		let start_offset = (stdout.len() as i32) - area.height as i32 + 2;
		let mut start: usize = 0;
		if start_offset > 0 { start = start_offset as usize; }
		let backscroll = stdout[start..].to_vec();
		frame.render_widget(
			Paragraph::new(Text::from(backscroll))
			.block(Block::default()
			       .borders(Borders::ALL)
			       .border_type(BorderType::Plain)
			       .border_style(Style::default().fg(Color::Blue)),
			),
			area,
		);
	}
	/// Provides the contents of the PLANQ's stdout as a set of formatted Line for ratatui
	pub fn get_stdout_as_lines(&self) -> Vec<Line> {
		let mut output: Vec<Line> = Vec::new();
		if self.stdout.is_empty() { return output; }
		for msg in self.stdout.iter() {
			output.push(msg.clone().into());
		}
		output
	}
	/// Handler for executing the shift into Idle mode; does a little bit of cleanup as part of the process
	pub fn idle_mode(&mut self, msglog: &mut MessageLog) {
		//self.stdout.push(Message::new(0, 0, "planq".to_string(), "".to_string()));
		//self.stdout.push(Message::new(0, 0, "planq".to_string(), "".to_string()));
		msglog.tell_planq(" ");
		self.cpu_mode = PlanqCPUMode::Idle;
	}
}

/// BEVY: Provides the Bevy-backed tools for doing things on the PLANQ involving time intervals
/// That is, this represents a 'process' or task within the PLANQ that needs processing time to complete
#[derive(Component, Clone, Debug, Default, Reflect)]
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

/// Defines the set of operating modes in the PLANQ's firmware
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum PlanqCPUMode {
	#[default]
	Idle,
	Error(u32),
	Startup,
	Shutdown,
	Working,
	Offline,
}
impl std::fmt::Display for PlanqCPUMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
/// Defines the full set of user commands that can actually be executed on the PLANQ
#[derive(Clone, Debug, Default, PartialEq, Eq, Reflect, EnumIter)]
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
impl std::fmt::Display for PlanqCmd {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match *self {
			PlanqCmd::NoOperation => { write!(f, "(NoOperation)") }
			PlanqCmd::Error(_) => { write!(f, "(Error)") }
			PlanqCmd::Help => { write!(f, "help") }
			PlanqCmd::Shutdown => { write!(f, "shutdown") }
			PlanqCmd::Reboot => { write!(f, "reboot") }
			PlanqCmd::Connect(_) => { write!(f, "connect") }
			PlanqCmd::Disconnect => { write!(f, "disconnect") }
		}
	}
}

//  ###: EVENTS
/// Describes a PLANQ-specific event, ie an event connected to its logic
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub struct PlanqEvent {
	pub etype: PlanqEventType,
}
impl PlanqEvent {
	pub fn new(new_type: PlanqEventType) -> PlanqEvent {
		PlanqEvent {
			etype: new_type,
		}
	}
}
impl Event for PlanqEvent {
	// This is required here to make the PlanqEvent compatible with Bevy's Event trait
}
/// Defines the set of control and input events that the Planq needs to handle
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum PlanqEventType {
	#[default]
	NullEvent,
	Startup,
	BootStage(u32),
	Shutdown,
	Reboot,
	GoIdle,
	CliOpen,
	CliClose,
	AccessLink,
	AccessUnlink,
}

//  ###: UTILITIES and COMPONENTS
/// Defines the PLANQ 'tag' component within Bevy
#[derive(Component, Copy, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Planq { }
impl Planq {
	pub fn new() -> Planq {
		Planq::default()
	}
}

// EOF
