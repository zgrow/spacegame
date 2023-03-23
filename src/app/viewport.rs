// viewport.rs
// Defines the Viewport object, which provides a roguelike-style grid-based Widget to ratatui

use crate::map::xy_to_index;
use crate::components::*;
use bevy::app::App;
use ratatui::{
	buffer::Buffer,
	widgets::{Widget, Block},
	layout::{Alignment, Rect},
	style::Style,
};

pub struct Viewport<'a> {
	ecs: &'a App,
	// these are the tui-rs attributes
	block: Option<Block<'a>>,
	style: Style,
	align: Alignment,
}
impl<'a> Widget for Viewport<'a> {
	fn render(mut self, area: Rect, buf: &mut Buffer) {
		// Ensure that the CameraView we're about to write into has the right size
		let view = self.ecs.world.get_resource::<CameraView>().unwrap();
		assert_eq!((area.width as i32, area.height as i32), (view.width, view.height), "CameraView and Widget::Viewport have mismatched size!");
		// Draw the border, if it exists
		let area = match self.block.take() {
			Some(b) => {
				let inner_area = b.inner(area);
				b.render(area, buf);
				inner_area
			}
			None => area,
		};
		// Don't continue if the area inside the border is too small
		if area.width < 1 || area.height < 1 {
			return;
		}
		// We are certain of a valid drawing area, so let's gooooo
		for map_y in area.top()..area.bottom() {        // Hooray
			for map_x in area.left()..area.right() {    // for 1:1 mapping!
				let index = xy_to_index(map_x.into(), map_y.into(), view.width);
				// FIXME: this doesn't include the modifiers
				let tilestyle = Style::default().fg(view.map[index].fg).bg(view.map[index].bg);
				buf.set_string(map_x, map_y, &view.map[index].glyph, tilestyle);
			}
		}
	}
}
impl <'a> Viewport<'a> {
	pub fn new(newworld: &'a App) -> Viewport<'a> {
		Viewport {
			ecs: newworld,
			block: None,
			style: Style::default(),
			align: Alignment::Left,
		}
	}
	// These are all chain methods to interconnect with tui-rs
	pub fn view(mut self, newworld: &'a App) -> Viewport<'a> {
		self.ecs = newworld;
		self
	}
	pub fn block(mut self, block: Block<'a>) -> Viewport<'a> {
		self.block = Some(block);
		self
	}
	pub fn style(mut self, style: Style) -> Viewport<'a> {
		self.style = style;
		self
	}
	pub fn alignment(mut self, align: Alignment) -> Viewport<'a> {
		self.align = align;
		self
	}
}

// EOF
