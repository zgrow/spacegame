// mod.rs
// Provides the heavy lifting for building maps without cluttering up main()

use crate::map::*;
mod rexpaint_map;
use rexpaint_map::RexMapBuilder;
use crate::components::Position;
use crate::item_builders::ItemType;

pub trait MapBuilder {
	fn build_map(&mut self);
	fn get_map(&self) -> Map;
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)>;
}
pub fn get_builder(selection: i32) -> Box<dyn MapBuilder>{
	match selection {
		1 => Box::new(RexMapBuilder::new()),
		_ => Box::new(DevMapBuilder::new())
	}
}
pub struct DevMapBuilder {
	map: Map,
	new_entys: Vec<(ItemType, Position)>,
}
impl MapBuilder for DevMapBuilder {
	fn build_map(&mut self) {
		// do the thing
		let new_width = 30;
		let new_height = 30;
		self.map = Map::new(new_width, new_height);
		let mut index;
		let x_max = new_width - 1;
		let y_max = new_height - 1;
		// Put up some walls and floors
		for y in 0..self.map.height {
			for x in 0..self.map.width {
				index = self.map.to_index(x, y);
				/*
				if y == 0 { self.map.tiles[index] = Tile::new_wall(); }
				else if y == y_max { self.map.tiles[index] = Tile::new_wall(); }
				else if x == 0 { self.map.tiles[index] = Tile::new_wall(); }
				else if x == x_max { self.map.tiles[index] = Tile::new_wall(); }
				else { self.map.tiles[index] = Tile::new_floor(); }
				*/
				if y == 0
				|| y == y_max
				|| x == 0
				|| x == x_max {
					self.map.tiles[index] = Tile::new_wall();
				}
			}
		}
		// Put in a single staircase
		index = self.map.to_index(5, 5);
		self.map.tiles[index] = Tile::new_stairway();
	}
	fn get_map(&self) -> Map {
		self.map.clone()
	}
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)> {
	    self.new_entys.clone()
	}
}
impl DevMapBuilder {
	pub fn new() -> DevMapBuilder {
		DevMapBuilder {
			map: Map::new(1, 1),
			new_entys: Vec::new(),
		}
	}
}
impl Default for DevMapBuilder {
	fn default() -> DevMapBuilder {
		DevMapBuilder::new()
	}
}
// EOF
