// map.rs
// Defines the gameworld's terrain and interlocks with some bracket-lib logic

// *** EXTERNAL LIBS
use std::fmt;
use std::fmt::Display;
use bracket_algorithm_traits::prelude::{Algorithm2D, BaseMap};
use bracket_geometry::prelude::*;
use bevy::prelude::{
	Entity,
	Reflect,
	ReflectResource,
	Resource,
};
use simplelog::*;

// *** INTERNAL LIBS
use crate::components::*;
use crate::mason::json_map::*;

// *** CONSTANTS
pub const MAPWIDTH: i32 = 80;
pub const MAPHEIGHT: i32 = 60;
pub const MAPSIZE: i32 = MAPWIDTH * MAPHEIGHT;

// *** METHODS
/// Reference method that allows calculation from an arbitrary width
pub fn xy_to_index(x: usize, y: usize, w: usize) -> usize {
	(y * w) + x
}

// *** STRUCTS
//   - PHYSICAL GAMEWORLD TYPES
///Decides whether the Tile is open terrain, a wall, et cetera
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub enum TileType {
	#[default]
	Vacuum,
	Floor,
	Wall,
	Stairway,
}
impl Display for TileType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let output = match self {
			TileType::Vacuum => { "vacuum" }
			TileType::Floor => { "floor" }
			TileType::Wall => { "wall" }
			TileType::Stairway => { "stairway" }
		};
		write!(f, "{}", output)
	}
}

///Represents a single position within the game world
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct Tile {
	pub ttype: TileType,
	contents: Vec<(i32, Entity)>, // Implemented as a stack with sorting on the first value of the tuple
	pub glyph: String,
	pub fg: u8, // Corresponds to indexed colors, ie the ANSI 0-15 basic set
	pub bg: u8, // Same as fg
	pub mods: u16, // Corresponds to ratatui's Modifier type; use Modifier::bits()/to_bits() for conversion
}
impl Default for Tile {
	fn default() -> Self {
		Tile::new_floor()
	}
}
impl Tile {
	pub fn tiletype(mut self, new_type: TileType) -> Self {
		self.ttype = new_type;
		self
	}
	pub fn glyph(mut self, new_glyph: &str) -> Self {
		self.glyph = new_glyph.to_string();
		self
	}
	pub fn colors(mut self, new_fg: u8, new_bg: u8) -> Self {
		self.fg = new_fg;
		self.bg = new_bg;
		self
	}
	pub fn mods(mut self, new_mods: u16) -> Self {
		self.mods = new_mods;
		self
	}
	/// Adds an entity to this Tile's list of contents
	pub fn add_to_contents(&mut self, new_item: (i32, Entity)) {
		// Always make sure there's at least a dummy Entity in the list, this could probably be more clever
		//if self.contents.is_empty() {
		//	self.contents.push((0, Entity::PLACEHOLDER));
		//}
		// Find the point in the stack where we'd like to insert the new Entity:
		// at the top of the list of Entities with the same priority, *not* the top of the entire stack
		// In general, if all the visible entities at a given point have the same priority,
		// then the entity that will be shown will be the one that most-recently entered that tile
		// If any entities have a higher priority, then those should be shown instead
		let mut insertion_index = 0;
		for enty in self.contents.iter() {
			if new_item.0 < enty.0 {
				insertion_index += 1;
			}
		}
		// Insert the new entity at the top of the items of the same priority, not the entire stack
		self.contents.insert(insertion_index, new_item);
	}
	/// Retrieves the Entity ID of the most-visible Entity at this Tile
	pub fn get_visible_entity(&self) -> Option<Entity> {
		if self.contents.is_empty() {
			return None;
		}
		Some(self.contents[0].1)
	}
	/// Retrieves the entire list of contents of this Tile
	pub fn get_all_contents(&self) -> Vec<Entity> {
		self.contents.iter().map(|x| x.1).collect()
	}
	/// Removes an Entity from this list of contents
	pub fn remove_from_contents(&mut self, target: Entity) {
		let mut index = 0;
		loop {
			if index >= self.contents.len() {
				break;
			}
			if self.contents[index].1 == target {
				self.contents.remove(index);
			}
			index += 1;
		}
	}
	/// Produces an 'empty space' tile
	pub fn new_vacuum() -> Tile {
		Tile {
			ttype: TileType::Vacuum,
			contents: Vec::new(),
			glyph: "★".to_string(),
			fg: 8,
			bg: 0,
			mods: 0,
		}
	}
	/// Produces a default 'floor' tile
	pub fn new_floor() -> Tile {
		Tile {
			ttype: TileType::Floor,
			contents: Vec::new(),
			glyph: ".".to_string(),
			fg: 8,
			bg: 0,
			mods: 0,
		}
	}
	/// Produces a default 'wall' tile
	pub fn new_wall() -> Tile {
		Tile {
			ttype: TileType::Wall,
			contents: Vec::new(),
			glyph: "╳".to_string(),
			fg: 7,
			bg: 0,
			mods: 0,
		}
	}
	/// Produces a default 'stairway' tile
	pub fn new_stairway() -> Tile {
		Tile {
			ttype: TileType::Stairway,
			contents: Vec::new(),
			glyph: "∑".to_string(),
			fg: 5,
			bg: 0,
			mods: 0,
		}
	}
}

///Represents a single layer of physical space in the game world
#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct Map {
	pub tiles: Vec<Tile>,
	pub width: usize,
	pub height: usize,
	pub revealed_tiles: Vec<bool>,
	pub visible_tiles: Vec<bool>,
	pub blocked_tiles: Vec<bool>,
	pub opaque_tiles: Vec<bool>,
}
impl Map {
	/// Generates a map from the default settings
	pub fn new(new_width: usize, new_height: usize) -> Map {
		let map_size = new_width * new_height;
		Map {
			tiles: vec![Tile::default(); map_size],
			width: new_width,
			height: new_height,
			revealed_tiles: vec![false; map_size],
			visible_tiles: vec![false; map_size],
			blocked_tiles: vec![false; map_size],
			opaque_tiles: vec![false; map_size],
		}
	}
	/// Converts an x, y pair into a tilemap index using the given map's width
	pub fn to_index(&self, x: i32, y: i32) -> usize {
		// fun fact: Rust will barf and crash on an overflow error if usizes are used here
		// okay but will it tho???
		// ... yes, it DEFINITELY will ( TT n TT)
		((y * self.width as i32) + x) as usize
	}
	/// Returns true if the tiletype is Wall
	pub fn is_occupied(&self, target: Position) -> bool {
		let index = self.to_index(target.x, target.y);
		if self.tiles[index].ttype == TileType::Wall { return true }
		false
	}
	/// Walks through the map and populates the blocked_tiles and opaque_tiles maps according to the TileTypes
	pub fn update_tilemaps(&mut self) {
		for (index, tile) in self.tiles.iter_mut().enumerate() {
			self.blocked_tiles[index] = tile.ttype == TileType::Wall;
			self.opaque_tiles[index] = tile.ttype == TileType::Wall;
		}
	}
	/// Obtains the Tile data from the given position and creates a ScreenCell to display it
	pub fn get_display_tile(&self, target: Position) -> Tile {
		self.tiles[self.to_index(target.x, target.y)].clone()
	}
	/// Obtains whatever Entity is visible at the given Position, if any
	pub fn get_visible_entity_at(&self, target: Position) -> Option<Entity> {
		self.tiles[self.to_index(target.x, target.y)].get_visible_entity()
	}
	/// Retrieves the entire list of contents at the specified Position
	pub fn get_contents_at(&self, target: Position) -> Vec<Entity> {
		let index = self.to_index(target.x, target.y);
		self.tiles[index].get_all_contents()
	}
	/// Adds an Entity to the list of occupants at the specified Position
	pub fn add_occupant(&mut self, priority: i32, new_enty: Entity, posn: Position) {
		let index = self.to_index(posn.x, posn.y);
		self.tiles[index].add_to_contents((priority, new_enty));
		//debug!("added occupant {:?} to position {}", new_enty, posn);
	}
	/// Removes an Entity from the contents list at the given Position
	pub fn remove_occupant(&mut self, target: Entity, posn: Position) {
		let index = self.to_index(posn.x, posn.y);
		self.tiles[index].remove_from_contents(target);
		//debug!("removed occupant {:?} from position {}", target, posn);
	}
}
// bracket-lib uses the Algorithm2D and BaseMap traits for FOV and pathfinding
impl Algorithm2D for Map {
	fn dimensions(&self) -> Point {
		Point::new(self.width, self.height)
	}
	/*
	fn index_to_point2d(&self, idx: usize) -> Point {
		Point::new(idx % self.width as usize, idx / self.width as usize)
	}
	*/
}
impl BaseMap for Map {
	fn is_opaque(&self, index: usize) -> bool {
		self.opaque_tiles[index]
	}
	//fn get_available_exits(&self, index: usize) -> SmallVec<[(usize, f32); 10]> {
		// "Returns a vector of tile indices to which one can path from the index"
		// "Does not need to be contiguous (teleports OK); do NOT return current tile as an exit"
	//}
	//fn get_pathing_distance(&self, indexStart: usize, indexFinish: usize) _> f32 {
		// "Return the distance you would like to use for path-finding"
	//}
}

/// Provides movement between non-contiguous points in the Map, ie for stairs between z-levels, or teleporters, &c
/// NOTE: If the Portal is NOT bidirectional, then it will only allow transition from self.left to self.right;
/// ie in the directions established when building the Portal via from() and to()
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialOrd, Ord, Reflect)]
pub struct Portal {
	pub left: Position,
	pub right: Position,
	pub bidir: bool,
}
impl Portal {
	pub fn new() -> Portal {
		Portal::default()
	}
	pub fn from(mut self, from: Position) -> Portal {
		self.left = from;
		self
	}
	pub fn to(mut self, to: Position) -> Portal {
		self.right = to;
		self
	}
	pub fn twoway(mut self, setting: bool) -> Portal {
		self.bidir = setting;
		self
	}
	pub fn exit_from(self, target: Position) -> Position {
		if target == self.left {
			self.right
		} else if target == self.right && self.bidir {
			self.left
		} else {
			Position::INVALID
		}
	}
	pub fn has(self, target: Position) -> bool {
		self.left == target || self.right == target
	}
}
impl PartialEq for Portal {
	/// NOTE: Given two portals A and B, A == B if their sides match; however, the order does not matter, thus:
	/// A == B <-- A.left == B.left AND A.right == B.right, OR, A.left == B.right AND A.right == B.left
	/// Therefore, the setting for bidirectionality does not matter; if that condition is required, then use the strict
	/// equality trait, Eq, to obtain that information. This allows for better duplicate detection: if two Portals have
	/// 'mirrored' equal sides (A.l==B.r, A.r==B.l), then there's no need for both. In the case where a Portal
	/// is not bidirectional, we want to be 100% certain that access is being checked correctly.
	fn eq(&self, other: &Self) -> bool {
		(self.left == other.left && self.right == other.right) || (self.left == other.right && self.right == other.left)
	}
}

//   - LOGICAL SHIP TOPOLOGY
// These linked-list directed graph routines were transcribed from:
// https://smallcultfollowing.com/babysteps/blog/2015/04/06/modeling-graphs-in-rust-using-vector-indices/
// on October 12, 2023
// Each Room (aka node) carries an index to its first _outgoing_ Door (aka edge), if present
// This first Door can optionally contain the indices of all the other Doors available to this Room
// Each Door contains an index to its destination Room, and the list of its companion Doors
// This way, a 1:1 relation is enforced between Rooms (nodes) and Doors (edges), modelling a linked list
// But actual rooms and doors in-game can have an x:y relation, or even an implicit, which models the game world
/// Describes a node in the topology graph, a single Room which is composed of a set of Positions
pub type RoomIndex = usize;
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct GraphRoom {
	pub name: String,
	interior: Vec<Position>,
	first_outgoing_door: Option<RoomIndex>,
}
impl Default for GraphRoom {
	fn default() -> GraphRoom {
		GraphRoom {
			name: "blank_room".to_string(),
			interior: Vec::new(),
			first_outgoing_door: None,
		}
	}
}
impl From<JsonRoom> for GraphRoom {
	fn from(new_room: JsonRoom) -> Self {
		let mut point_list: Vec<Position> = Vec::new();
		for whye in new_room.corner[1]..new_room.corner[1] + new_room.height {
			for echs in new_room.corner[0]..new_room.corner[0] + new_room.width {
				point_list.push(Position::new(echs as i32, whye as i32, new_room.corner[2] as i32));
			}
		}
		GraphRoom {
			name: new_room.name.clone(),
			interior: point_list,
			first_outgoing_door: None
		}
	}
}
impl GraphRoom {
	/// Returns True if the specified Position is within the walls of the called Room
	pub fn contains(&self, target: Position) -> bool {
		self.interior.contains(&target)
	}
	pub fn set_interior_to(&mut self, new_interior: Vec<Position>) {
		self.interior = new_interior;
	}
}
/// Describes an edge in the topology graph, a connection between two GraphRooms
pub type DoorIndex = usize;
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct GraphDoor {
	pub name: String,
	pub from: Position,
	pub to: Position,
	target: RoomIndex,
	next_outgoing_door: Option<DoorIndex>,
}
impl Default for GraphDoor {
	fn default() -> GraphDoor {
		GraphDoor {
			name: "blank_door".to_string(),
			from: Position::default(),
			to: Position::default(),
			target: 0,
			next_outgoing_door: None,
		}
	}
}
#[derive(Resource, Clone, Debug, Reflect)]
pub struct Successors<'a> {
	graph: &'a ShipGraph,
	current_door_index: Option<DoorIndex>,
}
impl<'a> Iterator for Successors<'a> {
	type Item = RoomIndex;
	fn next(&mut self) -> Option<RoomIndex> {
		match self.current_door_index {
			None => None,
			Some(door_num) => {
				let door = &self.graph.doors[door_num];
				self.current_door_index = door.next_outgoing_door;
				Some(door.target)
			}
		}
	}
}
#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct ShipGraph {
	pub rooms: Vec<GraphRoom>,
	pub doors: Vec<GraphDoor>,
}
impl ShipGraph {
	pub fn connect(&mut self, go_from: RoomIndex, go_to: RoomIndex) {
		let door_index = self.doors.len();
		let room_data = &mut self.rooms[go_from];
		self.doors.push(GraphDoor {
			target: go_to,
			next_outgoing_door: room_data.first_outgoing_door,
			..GraphDoor::default()
		}); // the other values not defined above will be defaults
		room_data.first_outgoing_door = Some(door_index);
	}
	pub fn add_room(&mut self, new_room: GraphRoom) -> RoomIndex {
		let index = self.rooms.len();
		self.rooms.push(new_room);
		index
	}
	/// Provides a recursive iterator that traverses the ShipGraph by links
	pub fn successors(&self, source: RoomIndex) -> Successors {
		let first_outgoing_door = self.rooms[source].first_outgoing_door;
		Successors { graph: self, current_door_index: first_outgoing_door }
	}
	/// Tests whether the specified Room is listed in the layout
	pub fn contains(&self, target: String) -> Option<usize> {
		for (index, room) in self.rooms.iter().enumerate() {
			if room.name == target {
				return Some(index);
			}
		}
		None
	}
	pub fn get_room_name(&self, target: Position) -> Option<String> {
		for room in &self.rooms {
			if room.contains(target) {
				return Some(room.name.clone());
			}
		}
		None
	}
}

//   - THE WORLD MODEL
/// Represents the entire stack of Maps that comprise a 3D space
#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct Model {
	pub levels: Vec<Map>,
	pub layout: ShipGraph,
	// WARN: DO NOT CONVERT THIS TO A HASHMAP OR BTREEMAP
	// Bevy's implementation of hashing and reflection makes this specific kind of Hashmap usage
	// *ineligible* for correct save/load via bevy_save; in short, the HashMap *itself* cannot be hashed,
	// so bevy_save shits itself and reports an "ineligible for hashing" error without any other useful info
	//pub portals: BTreeMap<Position, Position>,
	//pub portals: HashMap<Position, Position>,
	//pub portals: HashMap<(i32, i32, i32), (i32, i32, i32)> // Cross-level linkages
	//portals: Vec<(Position, Position)>,
	portals: Vec<Portal>,
}
impl Model {
	/// Sets up a linkage between two x,y,z positions, even on the same level
	/// If 'bidir' is true, then the portal will be made two-way
	// NOTE: may need more fxns for remove_portal, &c
	pub fn add_portal(&mut self, left: Position, right: Position, bidir: bool) {
		// Check if the portal exists already
		// If not, add the portal
		// If bidir, add the reverse portal as well
		self.portals.push(Portal::new().from(left).to(right).twoway(bidir));
		self.portals.sort(); // Helps prevent duplication and speeds up retrieval
	}
	pub fn get_exit(&mut self, entry: Position) -> Option<Position> {
		// if the position belongs to a portal in the list, return its destination
		// otherwise, return a None
		let portal = self.portals.iter().find(|p| p.has(entry)).map(|portal| portal.exit_from(entry));
		if let Some(Position::INVALID) = portal {
			None
		} else {
			portal
		}
	}
	pub fn add_contents(&mut self, posns: Vec<Position>, priority: i32, enty: Entity) {
		debug!("add_contents: {:?} for enty {:?} at priority {}", posns, enty, priority);
		for posn in posns {
			self.levels[posn.z as usize].add_occupant(priority, enty, posn);
		}
	}
	pub fn remove_contents(&mut self, posns: Vec<Position>, enty: Entity) {
		debug!("remove_contents: {:?} for enty {:?}", posns, enty);
		for posn in posns {
			self.levels[posn.z as usize].remove_occupant(enty, posn);
		}
	}
	pub fn get_contents_at(&self, target: Position) -> Vec<Entity> {
		self.levels[target.z as usize].get_contents_at(target)
	}
}


// EOF
