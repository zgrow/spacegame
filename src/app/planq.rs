// planq.rs
// Provides the handling and abstractions for the player's PLANQ

use ratatui::buffer::*;
use ratatui::layout::*;
use ratatui::layout::Rect;
use ratatui::widgets::*;
use ratatui::style::*;
use bevy::prelude::*;

/// Defines the Planq widget for ratatui: provides outputs directly from the Planq
/// as opposed to the CameraView, inventory display, &c, which use other Widgets
pub struct PlanqStatus<'a> {
	data: Vec<String>,
	block: Option<Block<'a>>,
	style: Style,
	align: Alignment,
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

/// Defines the Planq component within Bevy
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Planq { }

/// Defines the Planq settings/controls (interface bwn my GameEngine class & Bevy)
#[derive(Resource, FromReflect, Reflect, Eq, PartialEq, Clone, Debug, Default)]
#[reflect(Resource)]
pub struct PlanqSettings {
	pub is_running: bool,
	pub is_carried: bool, // true if the planq is in the player's inventory
	pub action_mode: PlanqActionMode,
	pub output_1_enabled: bool,
	pub output_2_enabled: bool,
	pub show_inventory: bool,
	pub inventory_list: Vec<Entity>,
}
impl PlanqSettings {
	pub fn new() -> PlanqSettings {
		PlanqSettings {
			is_running: true,
			is_carried: true, // TODO: set this to detect actual carried status
			action_mode: PlanqActionMode::Default,
			output_1_enabled: true,
			output_2_enabled: false,
			show_inventory: false,
			inventory_list: Vec::new(),
		}
	}
	pub fn inventory_toggle(&mut self) {
		if self.show_inventory == false { self.show_inventory = true; }
		else { self.show_inventory = false; }
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
}

// EOF
