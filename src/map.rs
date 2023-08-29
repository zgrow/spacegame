// map.rs
// Defines the gameworld's terrain and interlocks with some bracket-lib logic

// *** EXTERNAL LIBS
use bevy::utils::HashMap;
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
	pub fg: u8,
	pub bg: u8,
	pub mods: String,
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
			mods: "".to_string(),
		}
	}
	/// Produces a default 'floor' tile
	pub fn new_floor() -> Tile {
		Tile {
			ttype: TileType::Floor,
			glyph: ".".to_string(),
			fg: 8,
			bg: 0,
			mods: "".to_string(),
		}
	}
	/// Produces a default 'wall' tile
	pub fn new_wall() -> Tile {
		Tile {
			ttype: TileType::Wall,
			glyph: "╳".to_string(),
			fg: 7,
			bg: 0,
			mods: "".to_string(),
		}
	}
	/// Produces a default 'stairway' tile
	pub fn new_stairway() -> Tile {
		Tile {
			ttype: TileType::Stairway,
			glyph: "∑".to_string(),
			fg: 5,
			bg: 0,
			mods: "".to_string(),
		}
	}
	// Produces a default 'doorway' 
}
///Represents a single layer of physical space in the game world
#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct Map {
	pub tiles: Vec<Tile>,
	pub width: i32,
	pub height: i32,
	pub revealed_tiles: Vec<bool>,
	pub visible_tiles: Vec<bool>,
	pub blocked_tiles: Vec<bool>,
	pub opaque_tiles: Vec<bool>,
}
impl Map {
	/// Generates a map from the default settings
	pub fn new(new_width: i32, new_height: i32) -> Map {
		let map_size: usize = (new_width * new_height) as usize;
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
		((y * self.width) + x) as usize
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
	pub portals: HashMap<(i32, i32, i32), (i32, i32, i32)> // Cross-level linkages
}
impl Model {
	/// Sets up a linkage between two x,y,z positions, even on the same level
	/// If 'bidir' is true, then the portal will be made two-way
	pub fn add_portal(&mut self, left: (i32, i32, i32), right: (i32, i32, i32), bidir: bool) {
		self.portals.insert(left, right);
		if bidir {
			self.portals.insert(right, left);
		}
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
