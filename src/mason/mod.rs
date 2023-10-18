// mason/mod.rs
// Provides the heavy lifting for building maps without cluttering up main()

use std::fs::File;
use std::io::BufReader;
use crate::artisan::ItemType;
use crate::components::Position;
use crate::map::*;
pub mod rexpaint_loader;
mod rexpaint_map;
use rexpaint_map::RexMapBuilder;
pub mod json_map;
use json_map::*;
use simplelog::*;

pub trait WorldBuilder {
	fn build_world(&mut self);
	fn get_model(&self) -> Model;
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)>;
}
/// Loads a worldmodel from a pregenerated JSON file and sets it up for gameplay
pub fn get_world_builder() -> Box<dyn WorldBuilder> {
	Box::<JsonWorldBuilder>::default()
}

// *** JSONBUILDER
#[derive(Default)]
pub struct JsonWorldBuilder {
	model: Model,
	new_entys: Vec<(ItemType, Position)>,
}
impl JsonWorldBuilder {
	pub fn load_json_file(&mut self, file_path: &str) {
		let file = File::open(file_path).unwrap();
		let reader = BufReader::new(file);
		let input_data: JsonBucket = match serde_json::from_reader(reader) {
			Ok(output) => output,
			Err(_) => JsonBucket::default(),
		};
		// Use the map lists to create the map stack and put it into the model
		for (z_posn, input_map) in input_data.map_list.iter().enumerate() {
			let mut new_map = Map::new(input_map.width, input_map.height);
			for (y_posn, line) in input_map.tilemap.iter().enumerate() {
				for (x_posn, tile) in line.chars().enumerate() {
					let index = new_map.to_index(x_posn as i32, y_posn as i32);
					let new_tile = match tile {
						' ' => { Tile::new_vacuum() }
						'#' => { Tile::new_wall() }
						'.' => { Tile::new_floor() }
						'=' => {
							self.new_entys.push((ItemType::Door, Position::new(x_posn as i32, y_posn as i32, z_posn as i32)));
							Tile::new_floor()
						}
						 _  => { Tile::new_vacuum() }
					};
					new_map.tiles[index] = new_tile;
				}
			}
			self.model.levels.push(new_map);
		}
		// 2: use the room list to create the topo graph of the layout
		// Iterate on all the rooms in the input list
		for room in input_data.room_list.iter() {
			let room_index: usize;
			// If the room already exists, use its room index; else make a new room
			if let Some(new_index) = self.model.layout.contains(room.name.clone()) {
				room_index = new_index;
			} else {
				room_index = self.model.layout.add_room((*room).clone().into());
			}
			debug!("* new room: {}: {:?}", room_index, room.exits);
			// Iterate on all the exits attached to this room
			for destination in &room.exits {
				//debug!("* dest: {:?}", destination);
				let dest_index: usize;
				if let Some(new_index) = self.model.layout.contains(destination.clone()) {
					// If the destination room already exists, get its room_index
					dest_index = new_index;
				} else if destination.contains("hallway") {
					// If it doesn't exist AND it's a hallway ( FIXME: irregular shape!) then make the hallway now
					let mut new_room = GraphRoom::default();
					new_room.name = destination.clone();
					dest_index = self.model.layout.add_room(new_room);
				} else {
					// If it doesn't exist, just make it now and get its index
					let new_room = input_data.room_list.iter().find(|x| x.name == *destination).unwrap();
					dest_index = self.model.layout.add_room(new_room.clone().into());
				}
				self.model.layout.connect(room_index, dest_index);
			}
		}
		// 3: use the portal list to create the list of ladders that need to be spawned
		for portal in input_data.ladder_list.iter() {
			// The tiles at the target positions need to be set to TileType::Stairway
			let left_side = Position::new(portal.points[0][0] as i32, portal.points[0][1] as i32, portal.points[0][2] as i32);
			let l_index = self.model.levels[left_side.z as usize].to_index(left_side.x, left_side.y);
			self.model.levels[left_side.z as usize].tiles[l_index] = Tile::new_stairway();
			let right_side = Position::new(portal.points[1][0] as i32, portal.points[1][1] as i32, portal.points[1][2] as i32);
			let r_index = self.model.levels[right_side.z as usize].to_index(right_side.x, right_side.y);
			self.model.levels[right_side.z as usize].tiles[r_index] = Tile::new_stairway();
			self.model.add_portal(left_side, right_side, true);
		}
	}
}
impl WorldBuilder for JsonWorldBuilder {
	fn build_world(&mut self) {
		JsonWorldBuilder::load_json_file(self, "resources/test_ship_v3.json");
	}
	fn get_model(&self) -> Model {
		self.model.clone()
	}
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)> {
		self.new_entys.clone()
	}
}

// *** MAPBUILDER
pub trait MapBuilder {
	fn build_map(&mut self);
	fn get_map(&self) -> Map;
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)>;
}
pub fn get_map_builder(selection: i32) -> Box<dyn MapBuilder>{
	match selection {
		1  => Box::new(RexMapBuilder::new()),
		2  => Box::new(JsonMapBuilder::new()),
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
		// The tracking indices, aka offsets
		let mut dx = -radius;
		let mut dy = 0;
		// The centerpoint of the circle
		let cx = 10;
		let cy = 10;
		// Begin placing walls (all tiles are Floor by default)
		loop {
			// place a tile at cx - x, cy + y
			index = self.map.to_index(cx - dx, cy + dy);
			self.map.tiles[index] = Tile::new_wall();
			// place a tile at cx - y, cy - x
			index = self.map.to_index(cx - dy, cy - dx);
			self.map.tiles[index] = Tile::new_wall();
			// place a tile at cx + x, cy - y
			index = self.map.to_index(cx + dx, cy - dy);
			self.map.tiles[index] = Tile::new_wall();
			// place a tile at cx + y, cy + x
			index = self.map.to_index(cx + dy, cy + dx);
			self.map.tiles[index] = Tile::new_wall();
			// radius := err
			radius = err;
			if radius <= dy {
				dy += 1;
				err += dy * 2 + 1;
			}
			if radius > dx || err > dy {
				dx += 1;
				err += dx * 2 + 1;
			}
			if dx >= 0 { break; } // do ... while x < 0
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
				index = self.map.to_index(x as i32, y as i32);
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
