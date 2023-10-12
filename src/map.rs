// map.rs
// Defines the gameworld's terrain and interlocks with some bracket-lib logic

// *** EXTERNAL LIBS
use std::fmt;
use std::fmt::Display;
use bracket_algorithm_traits::prelude::{Algorithm2D, BaseMap};
use bracket_geometry::prelude::*;
use bevy::prelude::*;

// *** INTERNAL LIBS
use crate::components::*;

// *** CONSTANTS
pub const MAPWIDTH: i32 = 80;
pub const MAPHEIGHT: i32 = 60;
pub const MAPSIZE: i32 = MAPWIDTH * MAPHEIGHT;

// *** METHODS
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
	/// Produces an 'empty space' tile
	pub fn new_vacuum() -> Tile {
		Tile {
			ttype: TileType::Vacuum,
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
	//  PRIVATE METHODS
	/* Returns true if the specified location is not blocked
	fn is_exit_valid(&self, x: i32, y: i32) -> bool {
		if x < 1 || x > self.width - 1
		|| y < 1 || y > self.height - 1 {
			return false;
		}
		let index = self.to_index(x, y);
		!self.blocked_tiles[index]
	}
	*/
}
/// Represents the entire stack of Maps that comprise a 3D space
#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct Model {
	pub levels: Vec<Map>,
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
/// NOTE: Given two portals A and B, A == B if their sides match; however, the order does not matter, thus:
/// A == B <-- A.left == B.left AND A.right == B.right, OR, A.left == B.right AND A.right == B.left
/// Therefore, the setting for bidirectionality does not matter; if that condition is required, then use the strict
/// equality trait, Eq, to obtain that information. This allows for better duplicate detection: if two Portals have
/// 'mirrored' equal sides (A.l==B.r, A.r==B.l), then there's no need for both. In the case where a Portal
/// is not bidirectional, we want to be 100% certain that access is being checked correctly.
impl PartialEq for Portal {
	fn eq(&self, other: &Self) -> bool {
		(self.left == other.left && self.right == other.right) || (self.left == other.right && self.right == other.left)
	}
}
/// Reference method that allows calculation from an arbitrary width
pub fn xy_to_index(x: usize, y: usize, w: usize) -> usize {
	(y * w) + x
}
// bracket-lib uses the Algorithm2D, BaseMap, and Point objects
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
// bracket-lib uses the BaseMap trait to do FOV calculation and pathfinding
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

// EOF
