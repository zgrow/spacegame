// worldmap.rs
// Defines the gameworld's terrain and interlocks with some bracket-lib logic

// ###: EXTERNAL LIBS
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
use bevy_turborand::*;

// ###: INTERNAL LIBS
use crate::components::*;
use crate::components::Color;
use crate::camera::*;
use crate::mason::logical_map::*;

// ###: CONSTANTS
pub const MAPWIDTH: i32 = 80;
pub const MAPHEIGHT: i32 = 60;
pub const MAPSIZE: i32 = MAPWIDTH * MAPHEIGHT;

// ###: COMPLEX TYPES
/// Reference method that allows calculation from an arbitrary width
pub fn xy_to_index(x: usize, y: usize, w: usize) -> usize {
	(y * w) + x
}

// ###: STRUCTS
//  ##: WorldModel
/// Represents the entire stack of Maps that comprise a 3D space
#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct WorldModel {
	pub levels: Vec<WorldMap>,
	pub layout: ShipGraph,
	/* WARN: DO NOT CONVERT THIS TO A HASHMAP OR BTREEMAP
	 * Bevy's implementation of hashing and reflection makes this specific kind of Hashmap usage
	 * *ineligible* for correct save/load via bevy_save; in short, the HashMap *itself* cannot be hashed,
	 * so bevy_save shits itself and reports an "ineligible for hashing" error without any other useful info
	 * All of these declarations are the bones of those who came before:
	 *pub portals: BTreeMap<Position, Position>,
	 *pub portals: HashMap<Position, Position>,
	 *pub portals: HashMap<(i32, i32, i32), (i32, i32, i32)> // Cross-level linkages
	 *portals: Vec<(Position, Position)>,
	 */
	// NOTE: The above may not be true with the conversion to moonshine_save from bevy_save; testing is needed
	portals: Vec<Portal>,
}
impl WorldModel {
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
	/// Retrieve the destination of a given Portal, if any
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
	/// Retrieve the tiletype of the given Position
	pub fn get_tiletype_at(&self, target: Position) -> TileType {
		let index = self.levels[target.z as usize].to_index(target.x, target.y);
		self.levels[target.z as usize].tiles[index].ttype
	}
	/// Adds the given Entity as an occupant at the specified positions, with the given priority
	pub fn add_contents(&mut self, posns: &Vec<Position>, priority: i32, enty: Entity) {
		trace!("add_contents: {:?} for enty {:?} at priority {}", posns, enty, priority); // DEBUG: log the call to add_contents
		for posn in posns {
			self.levels[posn.z as usize].add_occupant(priority, enty, *posn);
		}
	}
	/// Removes the given Entity from the occupancy list of the specified Tiles
	pub fn remove_contents(&mut self, posns: &Vec<Position>, enty: Entity) {
		trace!("remove_contents: {:?} for enty {:?}", posns, enty); // DEBUG: log the call to remove_contents
		for posn in posns {
			self.levels[posn.z as usize].remove_occupant(enty, *posn);
		}
	}
	/// Retrieves a list of all the occupants at the given Position
	pub fn get_contents_at(&self, target: Position) -> Vec<Entity> {
		self.levels[target.z as usize].get_contents_at(target)
	}
	/// Iterates on the contents list of every Tile in the WorldModel and validates it with the given Entity map
	pub fn reload_tile_contents(&mut self, enty_bodies: Vec<(Entity, Vec<Glyph>)>) {
		//eprintln!("* supplied ref_map: {:#?}", ref_map);
		eprintln!("* old_enty_bodies: {:#?}", enty_bodies);
		self.drop_all_tile_contents();
		for (enty, body) in enty_bodies.iter() {
			for glyph in body.iter() {
				self.add_contents(&vec![glyph.posn], 0, *enty);
			}
		}
	}
	pub fn drop_all_tile_contents(&mut self) {
		for deck in self.levels.iter_mut() {
			for tile in deck.tiles.iter_mut() {
				tile.contents = Vec::new();
			}
		}
	}
	/// Returns True if the Position contains an Entity with Obstructive, or if the Tiletype is a blocking type
	pub fn is_blocked_at(&self, target: Position) -> bool {
		trace!("* is_blocked_at({:?})", target); // DEBUG: log the call to is_blocked_at
		let index = self.levels[target.z as usize].to_index(target.x, target.y);
		self.levels[target.z as usize].blocked_tiles[index]
	}
	/// Returns a list of all Obstructive Entities at the given Position, optionally with LOS from a given observer
	pub fn get_obstructions_at(&self, targets: Vec<Position>, observer_enty: Option<Entity>) -> Option<Vec<(Position, Obstructor)>> {
		let mut block_list = Vec::new();
		let observer = observer_enty.unwrap_or(Entity::PLACEHOLDER);
		for posn in targets.iter() {
			if self.is_blocked_at(*posn) {
				trace!("* enty is_blocked_at {}", posn); // DEBUG: log where the entity's movement attempt was blocked
				// Seems like a safe assumption that the most-visible entity at a given position will be the one blocking it
				if let Some(observed) = self.levels[posn.z as usize].get_visible_entity_at(*posn) {
					// If any entities were observed at that location, add them to the output list
					// Remember, this if-condition is evaluated serially: by definition, if the compiler evaluates the RHS,
					// then the LHS was already observed to be false
					if observer == Entity::PLACEHOLDER || observer != observed {
						block_list.insert(0, (*posn, Obstructor::Actor(observed)));
					}
				} else { // just add the blocking object as a Tiletype
					let ttype = self.get_tiletype_at(*posn);
					block_list.insert(0, (*posn, Obstructor::Object(ttype)));
				}
			}
		}
		trace!("* blockers found: {:?}", block_list); // DEBUG: log all of the blocking entities that were discovered
		if !block_list.is_empty() {
			Some(block_list)
		} else {
			None
		}
	}
	/// Tries to find the specified room in the world model, and if successful, tries to obtain a spawnpoint within
	pub fn find_spawnpoint_in(&mut self, target_room: &str, template: SpawnTemplate, rng: &mut GlobalRng) -> Option<Vec<(String, Position)>> {
		trace!("* find_spawnpoint_in {} for {:?}", target_room, template); // DEBUG: log the call to find_spawnpoint_in
		if let Some(room_index) = self.layout.get_room_index(target_room) {
			//self.layout.rooms[room_index].debug_print(); // DEBUG: display the current layout map of the room
			return self.layout.rooms[room_index].find_open_space(template, rng);
		}
		None
	}
	/// Returns a list of Room names in the topology of the ship
	pub fn get_room_name_list(&self) -> Vec<String> {
		self.layout.get_room_list()
	}
	/// Sets the state of a specific Position on the blocking map
	pub fn set_blocked_state(&mut self, target: Position, state: bool) {
		self.levels[target.z as usize].set_blocked(target, state);
	}
	/// Sets the state of a specific Position on the opaque map
	pub fn set_opaque_state(&mut self, target: Position, state: bool) {
		self.levels[target.z as usize].set_opaque(target, state);
	}
}
//   ##: WorldMap
/// Represents a single layer of physical space in the game world
#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct WorldMap {
	pub tiles: Vec<Tile>,
	pub width: usize,
	pub height: usize,
	pub revealed_tiles: Vec<bool>,
	pub visible_tiles: Vec<bool>,
	pub blocked_tiles: Vec<bool>,
	pub opaque_tiles: Vec<bool>,
}
impl WorldMap {
	/// Generates a map from the default settings
	pub fn new(new_width: usize, new_height: usize) -> WorldMap {
		let map_size = new_width * new_height;
		WorldMap {
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
		//debug!("added occupant {:?} to position {}", new_enty, posn); // DEBUG: log the call to add_occupant
	}
	/// Removes an Entity from the contents list at the given Position
	pub fn remove_occupant(&mut self, target: Entity, posn: Position) {
		let index = self.to_index(posn.x, posn.y);
		self.tiles[index].remove_from_contents(target);
		//debug!("removed occupant {:?} from position {}", target, posn); // DEBUG: log the call to remove_occupant
	}
	/// Sets a particular Position to blocked or not in the blocked_tiles map
	pub fn set_blocked(&mut self, target: Position, state: bool) {
		let index = self.to_index(target.x, target.y);
		self.blocked_tiles[index] = state;
	}
	/// Sets a particular Position to opaque or not on the opaque_tiles map
	pub fn set_opaque(&mut self, target: Position, state: bool) {
		let index = self.to_index(target.x, target.y);
		self.opaque_tiles[index] = state;
	}
}
// bracket-lib uses the Algorithm2D and BaseMap traits for FOV and pathfinding
impl Algorithm2D for WorldMap {
	fn dimensions(&self) -> Point {
		Point::new(self.width, self.height)
	}
	/*
	fn index_to_point2d(&self, idx: usize) -> Point {
		Point::new(idx % self.width as usize, idx / self.width as usize)
	}
	*/
}
impl BaseMap for WorldMap {
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
//    #: Tile
/// Represents a single position within the game world
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct Tile {
	pub ttype: TileType,
	pub cell: ScreenCell,
	#[reflect(ignore)]
	contents: Vec<(i32, Entity)>, // Implemented as a stack with sorting on the first value of the tuple
}
impl Tile {
	pub fn tiletype(mut self, new_type: TileType) -> Self {
		self.ttype = new_type;
		self
	}
	pub fn glyph(mut self, new_glyph: &str) -> Self {
		self.cell.glyph = new_glyph.to_string();
		self
	}
	pub fn colors(mut self, new_fg: Color, new_bg: Color) -> Self {
		self.cell.fg = new_fg as u8;
		self.cell.bg = new_bg as u8;
		self
	}
	pub fn mods(mut self, new_mods: u16) -> Self {
		self.cell.modifier = new_mods;
		self
	}
	/// Adds one or more Entities to this Tile's list of contents
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
	/// Retrieves the entire list of contents of this Tile; returns an empty vector if there's nothing to see
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
				//debug!("Removing enty {:?}", target); // DEBUG: log the call to remove_from_contents
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
			cell: ScreenCell::new_from_str("★ grey black none"),
		}
	}
	/// Produces a default 'floor' tile
	pub fn new_floor() -> Tile {
		Tile {
			ttype: TileType::Floor,
			contents: Vec::new(),
			cell: ScreenCell::new_from_str(". grey black none"),
		}
	}
	/// Produces a default 'wall' tile
	pub fn new_wall() -> Tile {
		Tile {
			ttype: TileType::Wall,
			contents: Vec::new(),
			cell: ScreenCell::new_from_str("╳ white black none"),
		}
	}
	/// Produces a default 'stairway' tile
	pub fn new_stairway() -> Tile {
		Tile {
			ttype: TileType::Stairway,
			contents: Vec::new(),
			cell: ScreenCell::new_from_str("∑ white black none"),
		}
	}
	/// Removes everything from the contents of this Tile
	pub fn clear_contents(&mut self) {
		self.contents = Vec::new();
	}
}
impl Default for Tile {
	fn default() -> Self {
		Tile::new_floor()
	}
}
//    #: Portal
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

//  ###: SIMPLE TYPES AND HELPERS
//   ##: TileType
/// Decides whether the Tile is open terrain, a wall, et cetera
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
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
//   ##: Obstructor
/// Represents a 'thing' that is blocking movement by an Entity into a particular Tile;
/// could be an Entity or just a particular TileType
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum Obstructor {
	Actor(Entity),
	Object(TileType),
}
// EOF
