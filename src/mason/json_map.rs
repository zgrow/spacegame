// mason/json_map.rs
// Provides the logic for importing my custom spaceship map layouts from JSON format

//  ###: EXTERNAL LIBRARIES
use serde::{Deserialize, Serialize};

//  ###: INTERNAL LIBRARIES
use crate::mason::*;

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

//   ##: JsonBucket
/// Data structure that maps to the JSON as laid out in the map generator for fast deserialization
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct JsonBucket {
	pub map_list: Vec<JsonMap>,
	pub room_list: Vec<JsonRoom>,
	pub ladder_list: Vec<JsonPortal>,
}
//   ##: JsonRoom
/// A JSON-formatted representation of a room
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonRoom {
	pub name: String,
	pub exits: Vec<String>,
	pub corner: Vec<usize>,
	pub width: usize,
	pub height: usize,
	pub contents: Vec<(String, u32)>, // the name of the item and how many to spawn
}
impl Default for JsonRoom {
	fn default() -> JsonRoom {
		JsonRoom {
			name: "".to_string(),
			exits: Vec::new(),
			corner: Vec::new(),
			width: 0,
			height: 0,
			contents: Vec::new(),
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
//   ##: JsonPortal
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
//   ##: JsonMap
/// A JSON-formatted representation of a 'raw' tilemap
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonMap {
	pub tilemap: Vec<String>,
	pub width: usize,
	pub height: usize,
}
impl From<JsonMap> for WorldMap {
	fn from(input: JsonMap) -> Self {
		for jmap in input.tilemap {
			warn!("> From<JsonMap> for GameMap unimplemented! input: {:?}", jmap); // DEBUG: log this type conversion
		}
		WorldMap::default()
	}
}

// EOF
