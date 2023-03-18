// viewport.rs
// Provides a viewport onto a (larger) tilemap, such as in a roguelike

use super::Map;
use super::super::map::TileType;
use super::super::components::Position;
use bevy::app::App;
use ::tui::{
	buffer::Buffer,
	widgets::{Widget, Block},
	layout::{Alignment, Rect},
	style::Style,
};

pub struct Viewport<'a> {
	//map: &'a str,
	ecs: &'a App,
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
		// METHOD
		// get a ref to the map
		//let target = self.app.world.resource::<Map>();
		let map = self.ecs.world.resource::<Map>();
		// get a ref to the player's location
		let ppos = self.ecs.world.resource::<Position>();
		// get the size of the viewport
		// > equal to area.width, area.height, etc as calculated from above
		// calc the centerpoint of the viewport
		let centerpoint = ((area.width / 2) as i32, (area.height / 2) as i32);
		// calc the min/max x,y of the map to obtain using:
		//      (player_x - center_x, player_y - center_y)
		let minimum = (ppos.x - centerpoint.0, ppos.y - centerpoint.1);
		let maximum = (minimum.0 + area.width as i32, minimum.1 + area.height as i32);
		// begin drawing the map:
		let mut screen_y = 1;
		for target_y in minimum.1..maximum.1 {
			let mut screen_x = 1;
			for target_x in minimum.0..maximum.0 {
				let mut glyph = "░";
				if target_x > 0
				&& target_y > 0
				&& target_x < map.width
				&& target_y < map.height {
					let index = map.xy_to_index(target_x, target_y);
					if map.revealed_tiles[index] {
						glyph = match map.tilemap[index] {
							TileType::Floor => ".",
							TileType::Wall => "+",
						};
					} else {
						glyph = "_";
					}
				}
				buf.set_string(screen_x, screen_y, glyph, Style::default());
				screen_x += 1;
			}
			screen_y += 1;
		}
		// draw the renderables, etc
	}
}
impl <'a> Viewport<'a> {
	pub fn view(mut self, newworld: &'a App) -> Viewport<'a> {
		self.ecs = newworld;
		self
	}
	pub fn new(newworld: &'a App) -> Viewport<'a> {
		Viewport {
			ecs: newworld,
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
