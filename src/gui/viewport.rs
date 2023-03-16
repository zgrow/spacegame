// viewport.rs
// Provides a viewport onto a (larger) tilemap, such as in a roguelike

use ::tui::{
	buffer::Buffer,
	widgets::{Widget, Block},
	layout::{Alignment, Rect},
	style::Style,
};
//use crate::map::TileType;

pub struct Viewport<'a> {
	map: &'a str,
	// these are the tui-rs attributes
	block: Option<Block<'a>>,
	style: Style,
	align: Alignment,
}
impl<'a> Widget for Viewport<'a> {
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
		// Don't continue if the area inside the border is too small
		if area.width < 1 || area.height < 1 {
			return;
		}
		// We are certain of a valid drawing area, so let's get started
        buf.set_string(area.left(), area.top(), self.map, self.style);
	}
}
impl <'a> Viewport<'a> {
	pub fn view(mut self, newmap: &'a str) -> Viewport<'a> {
		self.map = newmap;
		self
	}
	pub fn new(newmap: &'a str) -> Viewport<'a> {
		Viewport {
			map: newmap,
			block: None,
			style: Default::default(),
			align: Alignment::Left,
		}
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
