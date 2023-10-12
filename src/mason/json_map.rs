// mason/json_map.rs
// Provides the logic for importing my custom spaceship map layouts from JSON format

use std::fs::File;
use std::io::BufReader;
use serde::{Deserialize, Serialize};
use crate::mason::*;
use crate::components::Position;
use crate::artisan::ItemType;
use simplelog::*;

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

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonPortal {
	pub name: String,
	pub points: Vec<Vec<usize>>,
	pub twoway: bool
}
#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRoom {
	pub name: String,
	pub corner: Vec<usize>,
	pub width: usize,
	pub height: usize
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonMap {
	pub tilemap: Vec<String>,
	pub width: usize,
	pub height: usize,
}
impl From<JsonMap> for Map {
	fn from(input: JsonMap) -> Self {
		for jmap in input.tilemap {
			debug!("{:?}", jmap);
		}
		Map::default()
	}
}
/// Data structure that maps to the JSON as laid out in the map generator for fast deserialization
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct JsonBucket {
	pub map_list: Vec<JsonMap>,
	pub room_list: Vec<JsonRoom>,
	pub ladder_list: Vec<JsonPortal>,
}
pub struct JsonMapBuilder {
	map: Map,
	new_entys: Vec<(ItemType, Position)>,
}

impl MapBuilder for JsonMapBuilder {
	/// Processes the loaded JSON file into the internal Map representation
	fn build_map(&mut self) {
		JsonMapBuilder::load_map_file(self)
	}
	/// Retrieves a copy of the constructed Map from this builder
	fn get_map(&self) -> Map {
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
			map: Map::new(1, 1),
			new_entys: Vec::new(),
		}
	}
	fn load_map_file(&mut self) {
		// FIXME: This uses a 'magic label' for the filename!
		let filename = "resources/test_ship_v3.json";
		load_json_map(filename);
	}
}

pub fn load_json_map(filename: &str) -> (Map, Vec<(ItemType, Position)>) {
	// METHOD
	// 1 for each level,
	//      - copy each row of the tilemap into a new Map
	// 2 use the provided Graph object to construct the logical map of the rooms and connections
	// 3 copy the list of portals out so that Mason can construct the ladders
	let file = File::open(filename).unwrap();
	let reader = BufReader::new(file);
	let value: JsonBucket = match serde_json::from_reader(reader) {
			Ok(output) => output,
			Err(e) => {debug!("{}", e); JsonBucket::default()},
	};
	let map = value.map_list;
	(map[0].clone().into(), Vec::new()) // DEBUG: how come this works? doesn't it overwrite the list of doors?
}

// EOF
