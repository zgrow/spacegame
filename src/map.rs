// map.rs
// Defines the gameworld's terrain and interlocks with some bracket-lib logic
use bracket_algorithm_traits::prelude::{Algorithm2D, BaseMap};
use bracket_geometry::prelude::*;
use ratatui::style::Color;
use bevy::prelude::*;

pub const MAPWIDTH: i32 = 80;
pub const MAPHEIGHT: i32 = 60;
pub const MAPSIZE: i32 = MAPWIDTH * MAPHEIGHT;

///Decides whether the Tile is open terrain, a wall, et cetera
#[derive(PartialEq, Copy, Clone, Debug, Default)]
pub enum TileType {
	#[default]
	Vacuum,
	Floor,
	Wall,
}
///Represents a single position within the game world
#[derive(PartialEq, Clone, Debug)]
pub struct Tile {
	pub ttype: TileType,
	pub glyph: String,
	pub fg: Color,
	pub bg: Color,
	pub mods: String,
}
impl Default for Tile {
	fn default() -> Tile {
		Tile {
			ttype: TileType::Vacuum,
			glyph: "❏".to_string(),
			fg: Color::Magenta,
			bg: Color::Black,
			mods: "".to_string(),
		}
	}
}
impl Tile {
	///Produces an 'empty space' tile
	pub fn new_vacuum() -> Tile {
		Tile {
			ttype: TileType::Vacuum,
			glyph: "★".to_string(),
			fg: Color::DarkGray,
			bg: Color::Black,
			mods: "".to_string(),
		}
	}
	///Produces a default 'floor' tile
	pub fn new_floor() -> Tile {
		Tile {
			ttype: TileType::Floor,
			glyph: ".".to_string(),
			fg: Color::DarkGray,
			bg: Color::Black,
			mods: "".to_string(),
		}
	}
	///Produces a default 'wall' tile
	pub fn new_wall() -> Tile {
		Tile {
			ttype: TileType::Wall,
			glyph: "+".to_string(),
			fg: Color::Blue,
			bg: Color::Black,
			mods: "".to_string(),
		}
	}
}
///Represents a single layer of physical space in the game world
#[derive(Clone, Debug, Resource)]
pub struct Map {
//	pub tilemap: Vec<TileType>,
	pub tiles: Vec<Tile>,
	pub width: i32,
	pub height: i32,
	pub revealed_tiles: Vec<bool>,
	pub visible_tiles: Vec<bool>,
}
impl Map {
	/// Generates a map from the default settings
	pub fn new(_new_depth: i32, new_width: i32, new_height: i32) -> Map {
		let map_size: usize = (new_width * new_height) as usize;
		Map {
//			tilemap: vec![TileType::Floor; map_size],
			tiles: vec![Tile::default(); map_size],
			width: new_width,
			height: new_height,
			//:FIXME: set these back to false when ready to implement these features!
			revealed_tiles: vec![true; map_size],
			visible_tiles: vec![true; map_size],
		}
	}
	/// Converts an x, y pair into a tilemap index using the given map's width
	pub fn to_index(&self, x: i32, y: i32) -> usize {
		// fun fact: Rust will barf and crash on an overflow error if usizes are used here
		((y * self.width) + x) as usize
	}
}
/// Reference method that allows calculation from an arbitrary width
pub fn xy_to_index(x: i32, y: i32, w: i32) -> usize {
	(y as usize * w as usize) + x as usize
}
// NOTE: the Algorithm2D, BaseMap, and Point objects all come out of bracket-lib
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
		self.tiles[index].ttype == TileType::Wall
	}
}

// EOF
