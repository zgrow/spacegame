// mason/mod.rs
// Provides the heavy lifting for building maps without cluttering up main()

use simplelog::*;
use std::fs::File;
use std::io::BufReader;
use crate::artisan::ItemType;
use crate::components::Position;
use crate::worldmap::*;
pub mod rexpaint_loader;
mod rexpaint_map;
use rexpaint_map::RexMapBuilder;
pub mod json_map;
use json_map::*;
pub mod logical_map;
use logical_map::*;
//use bevy_turborand::*;

type Qpoint = (f32, f32);

/// Returns a vector of Positions that describe a direct line/path between the two inputs
pub fn get_line(first: &Position, second: &Position) -> Vec<Position> {
	let alpha: Qpoint = (first.x as f32, first.y as f32);
	let beta: Qpoint = (second.x as f32, second.y as f32);
	let mut points = Vec::new();
	let enn = diagonal_distance(&alpha, &beta);
	let end = enn as i32;
	for step in 0..end {
		let tee = if enn == 0.0 { 0.0 } else { step as f32 / enn };
		let qpoint = round_point(&lerp_point(&alpha, &beta, tee));
		let posn = Position::new(qpoint.0 as i32, qpoint.1 as i32, first.z);
		//points.push(round_point(lerp_point(&alpha, &beta, tee)));
		points.push(posn);
	}
	points
}
pub fn diagonal_distance(alpha: &Qpoint, beta: &Qpoint) -> f32 {
	let dx = beta.0 - alpha.0;
	let dy = beta.1 - alpha.1;
	f32::max(dx.abs(), dy.abs())
}
pub fn round_point(input: &Qpoint) -> Qpoint {
	(input.0.round(), input.1.round())
}
pub fn lerp_point(alpha: &Qpoint, beta: &Qpoint, tee: f32) -> Qpoint {
	(lerp(alpha.0, beta.0, tee), lerp(alpha.1, beta.1, tee))
}
pub fn lerp(start: f32, end: f32, tee: f32) -> f32 {
	start * (1.0 - tee) + tee * end
}

pub trait WorldBuilder {
	fn build_world(&mut self);
	fn get_model(&self) -> Model;
	//fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)>;
	fn get_essential_item_requests(&self) -> Vec<(String, Position)>;
	fn get_additional_item_requests(&self) -> Vec<(String, String)>;
}
/// Loads a worldmodel from a pregenerated JSON file and sets it up for gameplay
pub fn get_world_builder() -> Box<dyn WorldBuilder> {
	Box::<JsonWorldBuilder>::default()
}

// ###: JSONBUILDER
#[derive(Default)]
pub struct JsonWorldBuilder {
	model: Model,
	new_entys: Vec<(ItemType, Position)>,
	enty_list: Vec<(String, Position)>,
	addtl_items: Vec<(String, String)>
}
impl JsonWorldBuilder {
	pub fn load_json_file(&mut self, file_path: &str) {
		//debug!("* opening input file at {}", file_path);
		let file = File::open(file_path).unwrap();
		let reader = BufReader::new(file);
		let input_data: JsonBucket = match serde_json::from_reader(reader) {
			Ok(output) => output,
			//Err(_) => JsonBucket::default(),
			//Ok(output) => {debug!("* output recvd: {:#?}", output); output},
			Err(msg) => {warn!("! failed to read input data: {}", msg); JsonBucket::default()},
		};
		// 1: Use the map lists to create the map stack and put it into the model
		let mut hallway_tiles: Vec<Vec<Position>> = Vec::new();
		let mut logical_door_list: Vec<Position> = Vec::new();
		let mut _furniture_requests: Vec<(String, String)> = Vec::new();
		for (z_posn, input_map) in input_data.map_list.iter().enumerate() {
			let mut new_map = GameMap::new(input_map.width, input_map.height);
			let mut current_hallway = Vec::new();
			for (y_posn, line) in input_map.tilemap.iter().enumerate() {
				for (x_posn, tile) in line.chars().enumerate() {
					let index = new_map.to_index(x_posn as i32, y_posn as i32);
					let new_tile = match tile {
						' ' => { Tile::new_vacuum() }
						'#' => { Tile::new_wall() }
						'.' => { Tile::new_floor() }
						',' => {
							current_hallway.push((x_posn, y_posn, z_posn).into());
							Tile::new_floor().glyph("x")
						}
						'=' => {
							logical_door_list.push((x_posn, y_posn, z_posn).into());
							self.new_entys.push((ItemType::Door, Position::new(x_posn as i32, y_posn as i32, z_posn as i32)));
							self.enty_list.push(("door".to_string(), (x_posn, y_posn, z_posn).into()));
							Tile::new_floor()
						}
						 _  => { Tile::new_vacuum() }
					};
					new_map.tiles[index] = new_tile;
				}
			}
			self.model.levels.push(new_map);
			hallway_tiles.push(current_hallway);
		}
		// 2: Use the room list to create the topo graph of the layout
		// Iterate on all the rooms in the input list
		for cur_room in input_data.room_list.iter() {
			let room_index: usize;
			// If the cur_room already exists, use its cur_room index; else make a new room
			if let Some(new_index) = self.model.layout.contains(cur_room.name.clone()) {
				room_index = new_index;
			} else {
				room_index = self.model.layout.add_room((*cur_room).clone().into());
			}
			// Iterate on all the exits attached to this room
			for destination in &cur_room.exits {
				let dest_index: usize;
				if let Some(new_index) = self.model.layout.contains(destination.clone()) {
					// If the destination cur_room already exists, get its room_index
					dest_index = new_index;
				} else if destination.contains("hallway") {
					// If it doesn't exist AND it's a hallway ( FIXME: irregular shape!) then make the hallway now
					let mut new_room = GraphRoom::default();
					new_room.name = destination.clone();
					new_room.set_interior_to(hallway_tiles[cur_room.z_level()].clone());
					dest_index = self.model.layout.add_room(new_room);
				} else {
					// If it doesn't exist, just make it now and get its index
					let new_room = input_data.room_list.iter().find(|x| x.name == *destination).unwrap();
					dest_index = self.model.layout.add_room(new_room.clone().into());
				}
				self.model.layout.connect(room_index, dest_index);
			}
			// Add the room's contents to the list of items that will need spawnpoints generated
			if !cur_room.contents.is_empty() {
				debug!("* contents of room {}: {:#?}", cur_room.name, cur_room.contents);
				for (item_name, qty) in cur_room.contents.iter() {
					for _ in 0..*qty {
						//furniture_requests.push((cur_room.name.clone(), item_name.clone()));
						self.addtl_items.push((cur_room.name.clone(), item_name.clone()));
					}
				}
			}
		}
		// 2.5: Use the logical door list to populate those tiles in the logical maps of each room
		for posn in logical_door_list.iter() {
			// Get the room which contains the given position
			// Change the position in the room to Occupied
			if let Some(room_name) = self.model.layout.get_room_name(*posn) {
				let room_index = self.model.layout.rooms.iter().position(|x| x.name == room_name).unwrap();
				self.model.layout.rooms[room_index].new_interior.insert(*posn, CellType::Closed);
				// FIXME: NEED to add Margin tiles around the door
			}
			self.model.layout.add_door_to_map_at(*posn);
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
			// Set the stairway positions in the logical room maps as occupied
			// FIXME: NEED to add Margin tiles around the stairs
			//let l_room_name = self.model.layout.get_room_name(left_side).unwrap();
			//let l_room_index = self.model.layout.rooms.iter().position(|x| x.name == l_room_name).unwrap();
			//self.model.layout.rooms[l_room_index].new_interior.insert(left_side, GraphCell::new(CellType::Closed));
			//let r_room_name = self.model.layout.get_room_name(right_side).unwrap();
			//let r_room_index = self.model.layout.rooms.iter().position(|x| x.name == r_room_name).unwrap();
			//self.model.layout.rooms[r_room_index].new_interior.insert(right_side, GraphCell::new(CellType::Closed));
			self.model.layout.add_stairs_to_map_at(left_side);
			self.model.layout.add_stairs_to_map_at(right_side);
			// Add the graph connection between the two rooms using the manual method
			self.model.add_portal(left_side, right_side, true);
		}
		// 4: Iterate on the rooms in the logical graph and generate a list of items that each room needs for decorations
		//for (room_name, item_name, item_pattern) in furniture_requests.iter() {
		//	let spawn_posn = self.model.find_spawn_area_in(room_name, item_pattern);
		//	self.enty_list.push(item_name, spawn_posn);
		//}
		// ###: DEBUG: a bunch of different output formats for mapgen feedback
		//for room in self.model.layout.rooms.iter() {
		//	debug!("* new room: {}", room.name);
		//	room.debug_print();
		//}
		//debug!("* new room: {}", cur_room.name.clone()); // DEBUG: print the generated room's logical map
		//self.model.layout.rooms[room_index].debug_print();
	}
}
impl WorldBuilder for JsonWorldBuilder {
	fn build_world(&mut self) {
		JsonWorldBuilder::load_json_file(self, "resources/test_ship_v3.json");
	}
	fn get_model(&self) -> Model {
		self.model.clone()
	}
	//fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)> {
	//	self.new_entys.clone()
	//}
	fn get_essential_item_requests(&self) -> Vec<(String, Position)> {
		self.enty_list.clone()
	}
	fn get_additional_item_requests(&self) -> Vec<(String, String)> {
		self.addtl_items.clone()
	}
}

// ###: MAPBUILDER
pub trait MapBuilder {
	fn build_map(&mut self);
	fn get_map(&self) -> GameMap;
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)>;
}
pub fn get_map_builder(selection: i32) -> Box<dyn MapBuilder>{
	match selection {
		1  => Box::new(RexMapBuilder::new()),
		//2  => Box::new(JsonMapBuilder::new()),
		68 => Box::new(DevMapBasement::new()),
		69 => Box::new(DevMapLobby::new()),
		_  => Box::new(DevMapBasement::new())
	}
}
/// Creates the top level of the dev testing map
pub struct DevMapLobby {
	map: GameMap,
	new_entys: Vec<(ItemType, Position)>,
}
impl MapBuilder for DevMapLobby {
	fn build_map(&mut self) {
		// do the thing
		// make a blank map of size 30x30 tiles
		let new_width = 30;
		let new_height = 30;
		self.map = GameMap::new(new_width, new_height);
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
	fn get_map(&self) -> GameMap {
		self.map.clone()
	}
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)> {
	    self.new_entys.clone()
	}
}
impl DevMapLobby {
	pub fn new() -> DevMapLobby {
		DevMapLobby {
			map: GameMap::new(1, 1),
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
	map: GameMap,
	new_entys: Vec<(ItemType, Position)>,
}
impl MapBuilder for DevMapBasement {
	fn build_map(&mut self) {
		// do the thing
		let new_width = 30;
		let new_height = 30;
		self.map = GameMap::new(new_width, new_height);
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
	fn get_map(&self) -> GameMap {
		self.map.clone()
	}
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)> {
	    self.new_entys.clone()
	}
}
impl DevMapBasement {
	pub fn new() -> DevMapBasement {
		DevMapBasement {
			map: GameMap::new(1, 1),
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
