// mason/json_map.rs
// Provides the logic for importing my custom spaceship map layouts from JSON format

use std::fs::File;
use std::io::BufReader;
use serde::{Deserialize, Serialize};
use crate::mason::*;
use crate::components::Position;
use crate::artisan::ItemType;
//use simplelog::*;

/* The format of the input json as of October 10, 2023:
 *  {
 *    level_X:
 *      width    - integer, width of entire tilemap
 *      height   - integer, height of entire tilemap
 *      tilemap  - array of strings, where each string.length == width, and tilemap.length == height
 *    level_Y:
 *      ...
 *    graph:
 *      rooms:
 *        name   - string
 *        corner - array of 3 integers (-> 1 Position)
 *        width  - integer, width of room incl walls(?)
 *        height - integer, height of room incl walls(?)
 *      portals:
 *        name   - string
 *        points - array of 2 arrays of 3 integers (-> 2 Positions)
 *        twoway - bool, determines if the portal can be used in the reverse direction
 *  }
 */
/* The hierarchy of the Model object:
 *  Model
 *    Maps
 *      tiles - Vec<Tile>
 *      width - i32
 *      height - i32
 *      ...
 *    Portals
 *      left - Position
 *      right - Position
 *      bidir - bool
 *  (new)
 *    Graph
 *      Rooms
 */

/// A JSON-formatted representation of a door or other room-connecting passageway
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonPortal {
	pub name: String,
	pub points: Vec<Vec<usize>>,
	//pub twoway: bool
}
impl Default for JsonPortal {
	fn default() -> JsonPortal {
		JsonPortal {
			name: "".to_string(),
			points: Vec::new(),
		}
	}
}

/// A JSON-formatted representation of a room
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonRoom {
	pub name: String,
	pub exits: Vec<String>,
	pub corner: Vec<usize>,
	pub width: usize,
	pub height: usize,
}
impl Default for JsonRoom {
	fn default() -> JsonRoom {
		JsonRoom {
			name: "".to_string(),
			exits: Vec::new(),
			corner: Vec::new(),
			width: 0,
			height: 0,
		}
	}
}
impl JsonRoom {
	pub fn new() -> JsonRoom {
		JsonRoom::default()
	}
	pub fn name(mut self, new_name: String) -> JsonRoom {
		self.name = new_name;
		self
	}
	pub fn exits(mut self, exit_list: Vec<String>) -> JsonRoom {
		self.exits = exit_list;
		self
	}
	pub fn corner(mut self, posn: Vec<usize>) -> JsonRoom {
		self.corner = posn;
		self
	}
	pub fn dims(mut self, new_width: usize, new_height: usize) -> JsonRoom {
		self.width = new_width;
		self.height = new_height;
		self
	}
	pub fn z_level(&self) -> usize {
		self.corner[2]
	}
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonMap {
	pub tilemap: Vec<String>,
	pub width: usize,
	pub height: usize,
}
impl From<JsonMap> for GameMap {
	fn from(input: JsonMap) -> Self {
		for jmap in input.tilemap {
			debug!("ooo {:?}", jmap);
		}
		GameMap::default()
	}
}
/// Data structure that maps to the JSON as laid out in the map generator for fast deserialization
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct JsonBucket {
	pub map_list: Vec<JsonMap>,
	pub room_list: Vec<JsonRoom>,
	pub ladder_list: Vec<JsonPortal>,
}
/// The Builder object that produces maps from JSON
#[derive(Default, Debug)]
pub struct JsonMapBuilder {
	map: GameMap,
	new_entys: Vec<(ItemType, Position)>,
}
impl MapBuilder for JsonMapBuilder {
	/// Processes the loaded JSON file into the internal Map representation
	fn build_map(&mut self) {
		JsonMapBuilder::load_map_file(self)
	}
	/// Retrieves a copy of the constructed Map from this builder
	fn get_map(&self) -> GameMap {
		self.map.clone()
	}
	/// Retrieves a list of entities that need to be spawned after the Map is instantiated
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)> {
		self.new_entys.clone()
	}
}
impl JsonMapBuilder {
	pub fn new() -> JsonMapBuilder {
		JsonMapBuilder {
			map: GameMap::new(1, 1),
			new_entys: Vec::new(),
		}
	}
	fn load_map_file(&mut self) {
		// FIXME: This uses a 'magic label' for the filename!
		let filename = "resources/test_ship_v3.json";
		load_json_map(filename);
	}
}

pub fn load_json_map(filename: &str) -> (GameMap, Vec<(ItemType, Position)>) {
	// METHOD
	// 1 for each level,
	//      - copy each row of the tilemap into a new Map
	// 2 use the provided Graph object to construct the logical map of the rooms and connections
	// 3 copy the list of portals out so that Mason can construct the ladders
	let file = File::open(filename).unwrap();
	let reader = BufReader::new(file);
	let value: JsonBucket = match serde_json::from_reader(reader) {
			Ok(output) => output,
			Err(e) => {debug!("* load_json_map error: {}", e); JsonBucket::default()},
	};
	let map = value.map_list;
	(map[0].clone().into(), Vec::new()) // DEBUG: how come this works? doesn't it overwrite the list of doors?
}

// EOF
