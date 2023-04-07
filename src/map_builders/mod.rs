// mod.rs
// Provides the heavy lifting for building maps without cluttering up main()

use crate::map::*;
mod rexpaint_map;
use rexpaint_map::RexMapBuilder;

pub trait MapBuilder {
	fn build_map(&mut self);
	fn get_map(&self) -> Map;
}

pub fn get_builder(selection: i32) -> Box<dyn MapBuilder>{
	match selection {
		1 => Box::new(RexMapBuilder::new()),
		_ => Box::new(DevMapBuilder::new())
	}
}

pub struct DevMapBuilder {
	map: Map,
}
impl MapBuilder for DevMapBuilder {
	fn build_map(&mut self) {
		// do the thing
		let new_width = 30;
		let new_height = 30;
		let mut map: Map = Map::new(1, new_width, new_height);
		let mut index;
		for x in 0..map.width {
			index = map.to_index(x, 0);
			map.tiles[index] = Tile::new_wall();
			index = map.to_index(x, new_height - 1);
			map.tiles[index] = Tile::new_wall();
		}
		for y in 0..map.height {
			index = map.to_index(0, y);
			map.tiles[index] = Tile::new_wall();
			index = map.to_index(new_width - 1, y);
			map.tiles[index] = Tile::new_wall();
		}
		index = map.to_index(1, 1);
		map.tiles[index] = Tile::new_stairway();
	}
	fn get_map(&self) -> Map {
		self.map.clone()
	}
}
impl DevMapBuilder {
	pub fn new() -> DevMapBuilder {
		DevMapBuilder {
			map: Map::new(1, 1, 1)
		}
	}
}
// EOF
