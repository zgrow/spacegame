// viewport.rs
// Provides a viewport onto a (larger) tilemap, such as in a roguelike

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
		let buf_y = 0;
		for map_y in 0..view.height {
			let buf_x = 0;
			for map_x in 0..view.width {
				let index = xy_to_index(map_x, map_y, view.width);
				buf.set_string(buf_x, buf_y, &view.map[index].glyph, self.style);
			}
		}
	}
}
impl <'a> Viewport<'a> {
	pub fn new(newworld: &'a App) -> Viewport<'a> {
		Viewport {
			ecs: newworld,
			block: None,
			style: Default::default(),
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
