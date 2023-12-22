// planq/monitor.rs
// Provides the logic that handles the status bars and their updates

// ###: EXTERNAL LIBRARIES
use std::collections::VecDeque;
use bevy::prelude::*;
use bevy_turborand::{DelegatedRng, GlobalRng};
use bevy::utils::HashMap;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::*;

// ###: INTERNAL LIBRARIES
use crate::planq::*;
use crate::sys::DurationFmtExt;

// ###: BEVY SYSTEMS
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
	let (_enty, p_body, p_desc) = if let Ok(value) = p_query.get_single() { value } else { return };
	let (_enty, q_device) = if let Ok(value) = q_query.get_single_mut() { value } else { return };
	// Iterate any active PlanqProcesses
	// These should be iterated locally here so that they are consistent from frame to frame; this is because
	//   Bevy's Systems implement a multithreading model that does NOT guarantee anything about consistent concurrency
	for (_enty, mut s_clock) in s_query.iter_mut() {
		if !s_clock.timer.finished() {
			s_clock.timer.tick(time.delta());
		}
	}
	// -- STATUS BARS
	for (_enty, mut s_clock) in s_query.iter_mut() {
		if s_clock.timer.finished() {
			// If the timer's finished, ie the job is complete,
			// go to the logic for that data source and perform an update
			// HashMap::entry(key: K) retrieves the key's corresponding entry for modification;
			// HashMap::and_modify(f: F) performs the modification via closure F
			let source_name = s_clock.source.clone(); // <- type String needed here to give to the HashMap
			match source_name.as_str() {
				"planq_mode"      => {
					monitor.raw_data.entry(source_name).and_modify(|x| *x = PlanqDataType::Text(planq.cpu_mode.to_string()));
				}
				"player_location" => {
					monitor.raw_data.entry(source_name).and_modify(|x| *x = PlanqDataType::Text(p_desc.locn.clone()));
				}
				"current_time"    => { // FIXME: this shows as a stopwatch instead of an actual clock
					let start_time_offset = Duration::new(2096, 789); // 12:34:56.789
					let current_time = time.elapsed() + start_time_offset;
					monitor.raw_data.entry(source_name).and_modify(|x| *x = PlanqDataType::Text(current_time.get_as_string()));
				}
				"planq_battery"   => {
					monitor.raw_data.entry(source_name).and_modify(|x| *x = PlanqDataType::Percent(q_device.batt_voltage as u32));
				}
				"test_line"       => {
					monitor.raw_data.entry(source_name)
						.and_modify(|x| *x = PlanqDataType::Decimal{numer: rng.i32(0..100), denom: 100});
				}
				"test_sparkline"  => {
					// This update method is 'backwards' to the others: instead of passing a new value to raw_data via entry(),
					//   we modify the raw_data's values directly using the mutable reference we obtained with get_mut()
					if let Some(PlanqDataType::Series(ref mut arr)) = monitor.raw_data.get_mut(&source_name) {
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
			s_clock.timer.tick(time.delta());
		}
	}
	// -- SIMPLE DATA
	// Refresh the planq's scrollback
	// TODO: optimize this to avoid doing a full copy of the log every single time
	planq.stdout = msglog.get_log_as_messages("planq", 0);
	// Get the player's location
	planq.player_loc = p_body.ref_posn;
}

//  ###: MAJOR FXNS
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
	pub fn watch(mut self, source: &str) -> Self {
		self.status_bars.push(source.to_string());
		self
	}
	// General
	/// Removes the specified source from the list of status_bars, thus removing it from the PLANQ
	/// Returns true if the source was successfully removed
	pub fn remove(mut self, source: &str) -> bool {
		if let Some(posn) = self.status_bars.iter().position(|x| x == source) {
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
					PlanqDataType::Text(text_input) => {
						let prefix = match source.as_str() {
							"planq_mode" => { "MODE: ".to_string() }
							"player_location" => { "LOCN: ".to_string() }
							"current_time" => { "TIME: ".to_string() }
							_ => { "".to_string() }
						};
						let remainder = area.width as usize - prefix.len() - 2;
						let line = PlanqMonitor::right_align(text_input, remainder);
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
							//let line = PlanqMonitor::right_align(pct.to_string() + "%", remainder);
							let line = PlanqMonitor::right_align(format!("{}{}", pct, "%").as_str(), remainder);
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
	fn right_align(input: &str, width: usize) -> String {
		if input.len() >= width { return input.to_string(); }
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
	pub fn source(mut self, source: &str) -> Self {
		self.source = source.to_string();
		self
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

// EOF
