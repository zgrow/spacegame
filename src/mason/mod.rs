// mason/mod.rs
// Provides the heavy lifting for building maps without cluttering up main()

use crate::artisan::ItemType;
use crate::components::Position;
use crate::map::*;
mod rexpaint_map;
use rexpaint_map::RexMapBuilder;

pub trait MapBuilder {
	fn build_map(&mut self);
	fn get_map(&self) -> Map;
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)>;
}
pub fn get_builder(selection: i32) -> Box<dyn MapBuilder>{
	match selection {
		1  => Box::new(RexMapBuilder::new()),
		68 => Box::new(DevMapBasement::new()),
		69 => Box::new(DevMapLobby::new()),
		_  => Box::new(DevMapBasement::new())
	}
}
/// Creates the top level of the dev testing map
pub struct DevMapLobby {
	map: Map,
	new_entys: Vec<(ItemType, Position)>,
}
impl MapBuilder for DevMapLobby {
	fn build_map(&mut self) {
		// do the thing
		// make a blank map of size 30x30 tiles
		let new_width = 30;
		let new_height = 30;
		self.map = Map::new(new_width, new_height);
		// set the index and its maximums
		let mut index;
		// Put up some walls and floors
		// Let's draw a square of radius 10
		let mut radius = 10;
		let mut err = 2 - 2 * radius;
		// The tracking indices
		let mut x = -radius;
		let mut y = 0;
		// The centerpoint of the circle
		let cx = 10;
		let cy = 10;
		// Begin placing walls (all tiles are Floor by default)
		loop {
			// place a tile at cx - x, cy + y
			index = self.map.to_index(cx - x, cy + y);
			self.map.tiles[index] = Tile::new_wall();
			// place a tile at cx - y, cy - x
			index = self.map.to_index(cx - y, cy - x);
			self.map.tiles[index] = Tile::new_wall();
			// place a tile at cx + x, cy - y
			index = self.map.to_index(cx + x, cy - y);
			self.map.tiles[index] = Tile::new_wall();
			// place a tile at cx + y, cy + x
			index = self.map.to_index(cx + y, cy + x);
			self.map.tiles[index] = Tile::new_wall();
			// radius := err
			radius = err;
			if radius <= y {
				y += 1;
				err += y * 2 + 1;
			}
			if radius > x || err > y {
				x += 1;
				err += x * 2 + 1;
			}
			if x >= 0 { break; } // do ... while x < 0
		}
		// Put in a single staircase
		index = self.map.to_index(7, 7);
		self.map.tiles[index] = Tile::new_stairway();
	}
	fn get_map(&self) -> Map {
		self.map.clone()
	}
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)> {
	    self.new_entys.clone()
	}
}
impl DevMapLobby {
	pub fn new() -> DevMapLobby {
		DevMapLobby {
			map: Map::new(1, 1),
			new_entys: Vec::new(),
		}
	}
}
impl Default for DevMapLobby {
	fn default() -> DevMapLobby {
		DevMapLobby::new()
	}
}
/// Creates the bottom level of the dev testing map
pub struct DevMapBasement {
	map: Map,
	new_entys: Vec<(ItemType, Position)>,
}
impl MapBuilder for DevMapBasement {
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
impl DevMapBasement {
	pub fn new() -> DevMapBasement {
		DevMapBasement {
			map: Map::new(1, 1),
			new_entys: Vec::new(),
		}
	}
}
impl Default for DevMapBasement {
	fn default() -> DevMapBasement {
		DevMapBasement::new()
	}
}
// EOF
