// planq.rs
// Provides all of the logic and handling for the player's PLANQ

// TODO: import the PlanqEvent stuff from OLDsrc/app/event.rs

use bevy::prelude::*;
use bevy::utils::*;
use tui_textarea::TextArea;

//  *** SYSTEMS
/// Allows us to run PLANQ updates and methods in their own thread, just like a real computer~
pub fn planq_update_system(mut commands: Commands,
	                mut ereader:    EventReader<GameEvent>,
	                mut preader:    EventReader<PlanqEvent>,
	                mut msglog:     ResMut<MessageLog>,
	                time:       Res<Time>,
	                mut planq:      ResMut<PlanqData>, // contains the PLANQ's settings and data storage
	                p_query: Query<(Entity, &Position), With<Player>>, // provides interface to player data
	                i_query: Query<(Entity, &Portable), Without<Position>>,
	                q_query: Query<(Entity, &Planq, &Device)>, // contains the PLANQ's component data
	                mut t_query: Query<(Entity, &mut PlanqProcess)>, // contains the set of all PlanqTimers
) {
	/* TODO: Implement level generation such that the whole layout can be created at startup from a
	 * tree of rooms, rather than by directly loading a REXPaint map; by retaining this tree-list
	 * of rooms in the layout, the PLANQ can then show the player's location as a room name
	 */
	// Update the planq's settings if there are any changes queued up
	let player = p_query.get_single().unwrap();
	let planq_enty = q_query.get_single().unwrap();
	let mut refresh_inventory = false;
	// Handle any new comms
	for event in ereader.iter() {
		match event.etype {
			// Player interaction events that need to be monitored
			ItemMove => { // The player (g)ot the PLANQ from somewhere external
				let econtext = event.context.as_ref().unwrap();
				if econtext.subject == player.0 {
					refresh_inventory = true;
					if econtext.object == planq_enty.0 {
						planq.is_carried = true;
					}
				}
			}
			ItemDrop => { // The player (d)ropped the PLANQ
				let econtext = event.context.as_ref().unwrap();
				if econtext.subject == player.0 { refresh_inventory = true; }
				if econtext.object == planq_enty.0 { planq.is_carried = false; }
			}
			ItemUse => { // The player (a)pplied the PLANQ
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
	for event in preader.iter() {
		match event.etype {
			// PLANQ system commands
			PlanqEventType::NullEvent => { /* do nothing */ }
			Startup => { planq.cpu_mode = PlanqCPUMode::Startup; } // covers the entire boot stage
			BootStage(lvl) => {
				planq.boot_stage = lvl;
			}
			Shutdown => { planq.cpu_mode = PlanqCPUMode::Shutdown; }
			Reboot => { /* do a Shutdown, then a Startup */ }
			GoIdle => { planq.cpu_mode = PlanqCPUMode::Idle; }
			CliOpen => {
				planq.show_cli_input = true;
				planq.action_mode = PlanqActionMode::CliInput;
			}
			CliClose => {
				// FIXME: need to clear the CLI's input buffer! might need to do this at the time of key input?
				planq.show_cli_input = false;
				planq.action_mode = PlanqActionMode::Default; // FIXME: this might be a bad choice
			}
			InventoryUse => {
				planq.inventory_toggle(); // display the inventory menu
				planq.action_mode = PlanqActionMode::UseItem;
			}
			InventoryDrop => {
				planq.inventory_toggle(); // display the inventory menu
				planq.action_mode = PlanqActionMode::DropItem;
			}
		}
	}
	// Update the PLANQData resources:
	// - Get the device hardware info
	if !planq.power_is_on && planq_enty.2.pw_switch {
		planq.power_is_on = planq_enty.2.pw_switch; // Update the power switch setting
		planq.output_1_enabled = true; // DEBUG:
		planq.cpu_mode = PlanqCPUMode::Startup; // Begin booting the PLANQ's OS
	}
	if planq.power_is_on && !planq_enty.2.pw_switch {
		planq.power_is_on = planq_enty.2.pw_switch; // Update the power switch setting
		planq.cpu_mode = PlanqCPUMode::Shutdown; // Initiate a shutdown
	}
	// HINT: Get the current battery voltage with planq_enty.2.batt_voltage
	// - Iterate any active PlanqProcesses
	for mut pq_timer in t_query.iter_mut() {
		if !pq_timer.1.timer.finished() {
			pq_timer.1.timer.tick(time.delta());
		}
	}
	// - Handle the Planq's CPU mode logic
	match planq.cpu_mode {
		PlanqCPUMode::Error(_) => { /* TODO: implement Error modes */ }
		PlanqCPUMode::Offline => { /* do nothing */ }
		PlanqCPUMode::Startup => {
			// do the boot process: send outputs, progress bars, the works
			// then kick over to PAM::Idle
			if !planq.proc_table.is_empty() {
				// if there are any running processes, check to see if they're done
				for id in planq.proc_table.clone() {
					let enty = t_query.get(id).unwrap();
					if enty.1.timer.just_finished() {
						match enty.1.outcome.etype {
							BootStage(lvl) => {
								planq.boot_stage = lvl;
							}
							PlanqEventType::GoIdle => { planq.cpu_mode = PlanqCPUMode::Idle; }
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
			// TODO: rewrite these messages to appear as a ratatui::Table instead of a Paragraph
			match planq.boot_stage {
				0 => {
					if planq.proc_table.is_empty() {
						eprintln!("running boot stage 0");
						msglog.tell_planq("GRAIN v17.6.823 'Cedar'".to_string());
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
							eprintln!("running boot stage 1");
							msglog.tell_planq("Hardware Status ... [OK]".to_string());
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
							eprintln!("running boot stage 2");
							msglog.tell_planq("Firmware Status ... [OK]".to_string());
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
							eprintln!("running boot stage 3");
							msglog.tell_planq("Bootloader Status ... [OK]".to_string());
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
							eprintln!("running boot stage 4");
							// HINT: p_ruler:  1234567890123456789012345678 -- currently 28 chars
							msglog.tell_planq("CellulOS 5 (v19.26.619_revB)".to_string());
							proc.1.outcome = PlanqEvent::new(PlanqEventType::NullEvent);
							planq.cpu_mode = PlanqCPUMode::Idle;
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
		}
		PlanqCPUMode::Idle => {
			// Display a cute graphic
		}
		PlanqCPUMode::Working => {
			// Display the outputs from the workloads
		}
	}
	// - Refill the planq's inventory list
	if refresh_inventory {
		planq.inventory_list = Vec::new();
		for item in i_query.iter().enumerate() {
			if item.1.1.carrier == player.0 {
				planq.inventory_list.push(item.1.0);
			}
		}
	}
	// - Refresh the planq's scrollback
	// TODO: optimize this to avoid doing a full copy of the log every single time
	planq.stdout = msglog.get_log_as_messages("planq".to_string(), 0);
	// - Get the player's location
	planq.player_loc = *player.1;
}

//  *** STRUCTURES
/// BEVY: Defines the Planq settings/controls (interface bwn my GameEngine class & Bevy)
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, FromReflect)]
#[reflect(Resource)]
pub struct PlanqData {
	pub power_is_on: bool, // true if the planq has been turned on
	pub boot_stage: u32,
	pub is_carried: bool, // true if the planq is in the player's inventory
	pub cpu_mode: PlanqCPUMode,
	pub action_mode: PlanqActionMode, // Provides player action context for disambiguation
	pub output_1_enabled: bool,
	pub out1_mode: PlanqOutputMode,
	pub output_2_enabled: bool,
	pub out2_mode: PlanqOutputMode,
	pub show_inventory: bool,
	pub inventory_list: Vec<Entity>,
	pub player_loc: Position,
	pub show_cli_input: bool,
	pub stdout: Vec<Message>,
	pub proc_table: Vec<Entity>, // The list of PlanqProcesses running in the Planq
}
impl PlanqData {
	pub fn new() -> PlanqData {
		PlanqData {
			power_is_on: false,
			boot_stage: 0,
			is_carried: false,
			cpu_mode: PlanqCPUMode::Offline,
			action_mode: PlanqActionMode::Default,
			output_1_enabled: false,
			out1_mode: PlanqOutputMode::Terminal,
			output_2_enabled: false,
			out2_mode: PlanqOutputMode::Idle,
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
	/// Renders the status bars of the PLANQ
	pub fn render_status_bars<B: Backend>(&mut self, frame: &mut Frame<'_, B>, area: Rect) {
		let mut planq_text = vec!["test string".to_string()]; // DEBUG:
		planq_text.push(format!("*D* x: {}, y: {}, z: {}",
		                        self.player_loc.x, self.player_loc.y, self.player_loc.z)); // DEBUG:
		planq_text.push("1234567890123456789012345678".to_string()); // DEBUG: ruler
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
			.block(Block::default()
			       .borders(Borders::ALL)
			       .border_style(Style::default().fg(Color::Blue)))
			.style(Style::default())
			.wrap(Wrap { trim: true }),
			area,
		);
	}
}
/// TUI-TEXTAREA/RATATUI: Defines the CLI input system and its logic
/// Note that tui-textarea is a part of the ratatui ecosystem, and therefore
/// is ineligible, *by definition*, for addition to the Bevy ecosystem
#[derive(Clone, Debug, Default)]
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
/// BEVY: Provides the Bevy-backed tools for doing things on the PLANQ involving time intervals
#[derive(Component, Clone, Debug, Default, Reflect, FromReflect)]
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

//  *** EVENTS
/// Describes a PLANQ-specific event, ie an event connected to its logic
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, FromReflect)]
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
/// Defines the set of control and input events that the Planq needs to handle
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, FromReflect)]
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
	InventoryUse,
	InventoryDrop,
}


//  *** UTILITIES and COMPONENTS
/// Defines the Planq 'tag' component within Bevy
#[derive(Component, Copy, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Planq { }
/// Provides context for certain actions (inventory use/drop, &c) that take secondary inputs
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Reflect, FromReflect)]
pub enum PlanqActionMode {
	#[default]
	Default,
	DropItem,
	UseItem,
	CliInput,
}
/// Defines the set of operating modes in the PLANQ's firmware
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Reflect, FromReflect)]
pub enum PlanqCPUMode {
	#[default]
	Idle,
	Error(u32),
	Startup,
	Shutdown,
	Working,
	Offline,
}
/// Defines the set of output modes for the PLANQ's dual output windows
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Reflect, FromReflect)]
pub enum PlanqOutputMode {
	#[default]
	Idle,
	InventoryChooser,
	Terminal,
	Settings,
}

// EOF
