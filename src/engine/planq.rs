// planq.rs
// Provides all of the logic and handling for the player's PLANQ

#![allow(clippy::too_many_arguments)]

use std::collections::VecDeque;
use bevy::{
	prelude::*,
	ecs::query::*,
	utils::*,
};
use ratatui::prelude::*;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui::widgets::Block;
use strum_macros::EnumIter;
use tui_textarea::TextArea;

use crate::{
	components::*,
	engine::{
		PlanqCmd::*,
		PlanqEventType::BootStage,
		*,
		event::*,
	},
};

//  ###: SYSTEMS
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
	let player = p_query.get_single().unwrap();
	let planq_enty = q_query.get_single_mut().unwrap();
	// Handle any new GameEvents we're interested in
	if !ereader.is_empty() {
		for event in ereader.iter() {
			let atype;
			if let GameEventType::PlayerAction(action) = event.etype {
				atype = action;
			} else {
				continue;
			}
			match atype {
				// Player interaction events that need to be monitored
				ActionType::MoveItem => { // The player (g)ot the PLANQ from somewhere external
					let econtext = event.context.as_ref().unwrap();
					planq.is_carried = econtext.subject == player.0 && econtext.object == planq_enty.0;
				}
				ActionType::DropItem => { // The player (d)ropped the PLANQ
					let econtext = event.context.as_ref().unwrap();
					if econtext.object == planq_enty.0 { planq.is_carried = false; }
				}
				ActionType::UseItem => { // The player (a)pplied the PLANQ
					let econtext = event.context.as_ref().unwrap();
					if econtext.subject == player.0
					&& econtext.object == planq_enty.0 {
						// Note that the Operable system already handles the ItemUse action for the
						// PLANQ: it allows the player to operate the power switch
						// This seems likely to change in the future to allow some better service
						// commands, like battery swaps or peripheral attachment
						msglog.tell_player("There is a faint 'click' as you press the PLANQ's power button.".to_string());
					}
				}
				_ => { }
			}
		}
	}
	// Handle all new PlanqEvents
	if !preader.is_empty() {
		for event in preader.iter() {
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
	if !planq.power_is_on && planq_enty.1.pw_switch {
		planq.power_is_on = planq_enty.1.pw_switch; // Update the power switch setting
		planq.show_terminal = true;
		planq.cpu_mode = PlanqCPUMode::Startup; // Begin booting the PLANQ's OS
	}
	if planq.power_is_on && !planq_enty.1.pw_switch {
		planq.power_is_on = planq_enty.1.pw_switch; // Update the power switch setting
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
					let enty = t_query.get(id).unwrap();
					if enty.1.timer.just_finished() {
						match enty.1.outcome.etype {
							BootStage(lvl) => {
								planq.boot_stage = lvl;
							}
							PlanqEventType::GoIdle => { planq.idle_mode(&mut msglog); }
							_ => { }
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
					if let Ok(mut proc) = proc_ref {
						if proc.1.timer.just_finished() {
							//debug!("¶ running boot stage {}", planq.boot_stage); // DEBUG: announce the current PLANQ boot stage
							msglog.boot_message(planq.boot_stage);
							// set its duration, if needed
							//proc.1.timer.set_duration(Duration::from_secs(5));
							// reset it
							proc.1.timer.reset(); // will be iterated on at next system run
							proc.1.outcome = PlanqEvent::new(PlanqEventType::BootStage(2));
						}
					}
				}
				2 => {
					if let Ok(mut proc) = proc_ref {
						if proc.1.timer.just_finished() {
							//debug!("¶ running boot stage {}", planq.boot_stage); // DEBUG: announce the current PLANQ boot stage
							msglog.boot_message(planq.boot_stage);
							// set its duration, if needed
							//proc.1.timer.set_duration(Duration::from_secs(5));
							// reset it and start it
							proc.1.timer.reset(); // will be iterated on at next system run
							proc.1.outcome = PlanqEvent::new(PlanqEventType::BootStage(3));
						}
					}
				}
				3 => {
					if let Ok(mut proc) = proc_ref {
						if proc.1.timer.just_finished() {
							//debug!("¶ running boot stage {}", planq.boot_stage); // DEBUG: announce the current PLANQ boot stage
							msglog.boot_message(planq.boot_stage);
							// set its duration, if needed
							//proc.1.timer.set_duration(Duration::from_secs(5));
							// reset it and start it
							proc.1.timer.reset(); // will be iterated on at next system run
							proc.1.outcome = PlanqEvent::new(PlanqEventType::BootStage(4));
						}
					}
				}
				4 => {
					if let Ok(mut proc) = proc_ref {
						if proc.1.timer.just_finished() {
							//debug!("¶ running boot stage {}", planq.boot_stage); // DEBUG: announce the current PLANQ boot stage
							msglog.boot_message(planq.boot_stage);
							proc.1.outcome = PlanqEvent::new(PlanqEventType::NullEvent);
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
	for mut proc in t_query.iter_mut() {
		if !proc.1.timer.finished() {
			proc.1.timer.tick(time.delta());
		}
	}
	// - Check for some edge cases and other things that we'd like to avoid
	if planq.is_carried && planq_enty.2.carrier != player.0 { planq.is_carried = false; }
	if !planq.is_carried && planq_enty.2.carrier == player.0 { planq.is_carried = true; }
}
/// Handles the PLANQ's output status bars and other such things
pub fn planq_monitor_system(time:        Res<Time>,
	                          mut rng:     ResMut<GlobalRng>,
	                          msglog:      ResMut<MessageLog>,
	                          mut planq:   ResMut<PlanqData>,
	                          mut monitor: ResMut<PlanqMonitor>,
	                          p_query:     Query<(Entity, &Body, &Description), With<Player>>,
	                          //mut q_query: Query<(Entity, &Device, &mut RngComponent), With<Planq>>,
	                          mut q_query: Query<(Entity, &Device), With<Planq>>,
	                          mut s_query: Query<(Entity, &mut DataSampleTimer)>,
) {
	if p_query.is_empty() { return; }
	if q_query.is_empty() { return; }
	let player = p_query.get_single().unwrap();
	let planq_enty = q_query.get_single_mut().unwrap();
	// Iterate any active PlanqProcesses
	// These should be iterated locally here so that they are consistent from frame to frame; this is because
	//   Bevy's Systems implement a multithreading model that does NOT guarantee anything about consistent concurrency
	for mut pq_timer in s_query.iter_mut() {
		if !pq_timer.1.timer.finished() {
			pq_timer.1.timer.tick(time.delta());
		}
	}
	// -- STATUS BARS
	for mut process in s_query.iter_mut() {
		if process.1.timer.finished() {
			let source_name = process.1.source.clone();
			match source_name.as_str() {
				"planq_mode"      => {
					monitor.raw_data.entry(source_name).and_modify(|x| *x = PlanqDataType::Text(planq.cpu_mode.to_string()));
				}
				"player_location" => {
					monitor.raw_data.entry(source_name).and_modify(|x| *x = PlanqDataType::Text(player.2.locn.clone()));
				}
				"current_time"    => { // FIXME: this shows as a stopwatch instead of an actual clock
					let start_time_offset = Duration::new(2096, 789); // 12:34:56.789
					let current_time = time.elapsed() + start_time_offset;
					monitor.raw_data.entry(source_name).and_modify(|x| *x = PlanqDataType::Text(current_time.get_as_string()));
				}
				"planq_battery"   => {
					monitor.raw_data.entry(source_name).and_modify(|x| *x = PlanqDataType::Percent(planq_enty.1.batt_voltage as u32));
				}
				"test_line"       => {
					monitor.raw_data.entry(source_name)
						.and_modify(|x| *x = PlanqDataType::Decimal{numer: rng.i32(0..100), denom: 100});
				}
				"test_sparkline"  => {
					// This update method is 'backwards' to the others: instead of passing a new value to raw_data via entry(),
					//   we modify the raw_data's values directly using the mutable reference we obtained with get_mut()
					let entry = monitor.raw_data.get_mut(&source_name).unwrap();
					if let PlanqDataType::Series(ref mut arr) = entry {
						arr.push_back(rng.u64(0..10));
						loop {
							if arr.len() >= 31 {
								arr.pop_front();
							} else {
								break;
							}
						}
					}
				}
				"test_gauge"      => {
					monitor.raw_data.entry(source_name)
						.and_modify(|x| *x = PlanqDataType::Percent(rng.u32(0..=100)));
				}
				_ => { error!("* unrecognized data source in planq_monitor_system: {}", source_name); } // DEBUG: announce a missing data source
			}
		} else {
			process.1.timer.tick(time.delta());
		}
	}
	// -- SIMPLE DATA
	// Refresh the planq's scrollback
	// TODO: optimize this to avoid doing a full copy of the log every single time
	planq.stdout = msglog.get_log_as_messages("planq".to_string(), 0);
	// Get the player's location
	planq.player_loc = player.1.ref_posn;
}

//  ###: STRUCTURES
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
		msglog.tell_planq(" ".to_string());
		self.cpu_mode = PlanqCPUMode::Idle;
	}
}
/// Handles the PLANQ's status bars, their settings, their inputs, &c
#[derive(Resource, Clone, Debug, PartialEq, Eq, Reflect)]
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
			if let Some(source_type) = self.raw_data.get(source) {
				match source_type {
					PlanqDataType::Text(text) => {
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
			status_bars: vec!["planq_battery".to_string(), "planq_mode".to_string(), "current_time".to_string(), "player_location".to_string()],
			raw_data: HashMap::from([("current_time".to_string(), PlanqDataType::Text("Initializing...".to_string())),
				                       ("planq_battery".to_string(), PlanqDataType::Percent(0)),
				                       ("planq_mode".to_string(), PlanqDataType::Text("Initializing...".to_string())),
				                       ("player_location".to_string(), PlanqDataType::Text("Initializing...".to_string())),
			]),
		}
	}
}
/// Defines the set of possible data types that a PLANQ's data source might provide
#[derive(Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub enum PlanqDataType {
	#[default]
	Null,
	Text(String), // Ideally this should be a Span or some other ratatui-compat type instead
	Integer(i32),
	Percent(u32),
	Decimal{numer: i32, denom: i32}, // Floating point numbers don't impl Eq, only PartialEq, so we have to use this pair of ints as a fractional representation instead
	Series(VecDeque<u64>),
}
/// TUI-TEXTAREA/RATATUI: Defines the CLI input system and its logic
/// Note that tui-textarea is a part of the ratatui ecosystem, and therefore
/// is ineligible, *by definition*, for addition to the Bevy ecosystem
#[derive(Clone, Default)]
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
/// Provides context for certain actions (inventory use/drop, &c) that take secondary inputs
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum PlanqActionMode {
	#[default]
	Default,
	DropItem,
	UseItem,
	CliInput,
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

/// RATATUI: Defines the Planq status widget for ratatui, provides outputs directly from the Planq
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
		let textstyle = Style::default().fg(Color::White);
		// put the contents of self.data on the screen
		let mut y_index = area.top();
		for line in self.data {
			buf.set_string(area.left(), y_index, line, textstyle);
			y_index += 1;
		}
	}
}
/// Provides a means for setting regular intervals for the PLANQ's monitoring, so that we are not
/// forced to provide updates at the framerate (and possibly cause flickering, &c)
/// If no duration is specified, the DataSample source will always be updated
#[derive(Component, Clone, Debug, Default, Reflect)]
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
