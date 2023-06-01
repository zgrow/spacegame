// viewport.rs
// Defines the Viewport object, which provides a roguelike-style grid-based Widget to ratatui

use ratatui::style::Color::Indexed;
use crate::map::xy_to_index;
use crate::camera_system::CameraView;
use ratatui::{
	buffer::Buffer,
	widgets::{Widget, Block},
	layout::{Alignment, Rect},
	style::Style,
};

pub struct Viewport<'a> {
	source: &'a CameraView,
	// these are the tui-rs attributes
	block: Option<Block<'a>>,
	style: Style,
	align: Alignment,
}
impl<'a> Widget for Viewport<'a> {
	fn render(mut self, area: Rect, buf: &mut Buffer) {
		// Ensure that the CameraView we're about to write into has the right size
		assert_eq!((self.source.width, self.source.height), (area.width as i32, area.height as i32),
			       "CameraView and Widget::Viewport have mismatched sizes!");
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
		if area.width < 1 || area.height < 1
		|| self.source.map.is_empty() {
			return;
		}
		// We are certain of a valid drawing area, so let's gooooo
		for map_y in area.top()..area.bottom() {        // Hooray
			for map_x in area.left()..area.right() {    // for 1:1 mapping!
				let index = xy_to_index(map_x.into(), map_y.into(), self.source.width);
				// TODO: this doesn't include the style modifiers
				let tilestyle = Style::default().fg(Indexed(self.source.map[index].fg)).bg(Indexed(self.source.map[index].bg));
				buf.set_string(map_x, map_y, &self.source.map[index].glyph, tilestyle);
			}
		}
	}
}
impl <'a> Viewport<'a> {
	pub fn new(new_source: &'a CameraView) -> Viewport<'a> {
		Viewport {
			source: new_source,
			block: None,
			style: Style::default(),
			align: Alignment::Left,
		}
	}
	// These are all chain methods to interconnect with tui-rs
	pub fn view(mut self, new_source: &'a CameraView) -> Viewport<'a> {
		self.source = new_source;
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
