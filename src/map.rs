// map.rs
use bracket_algorithm_traits::prelude::{Algorithm2D, BaseMap};
use bracket_geometry::prelude::*;
use bevy::prelude::*;

pub const MAPWIDTH: i32 = 80;
pub const MAPHEIGHT: i32 = 60;
pub const MAPSIZE: i32 = MAPWIDTH * MAPHEIGHT;

#[derive(PartialEq, Copy, Clone, Debug, Default)]
pub enum TileType {
	#[default]
	Floor,
	Wall,
}
#[derive(Clone, Debug, Resource)]
///Represents a single layer of physical space in the game world
pub struct Map {
	pub tilemap: Vec<TileType>,
	pub width: i32,
	pub height: i32,
	//pub size: i32,
	pub revealed_tiles: Vec<bool>,
	pub visible_tiles: Vec<bool>,
}
impl Map {
	/// Generates a map from the default settings
	pub fn new(_new_depth: i32, new_width: i32, new_height: i32) -> Map {
		Map {
			width: new_width,
			height: new_height,
			//size: new_width * new_height,
			tilemap: vec![TileType::Floor; (new_width * new_height) as usize],
			//:FIXME: set these back to false when ready to implement these features!
			revealed_tiles: vec![true; (new_width * new_height) as usize],
			visible_tiles: vec![true; (new_width * new_height) as usize],
		}
	}
	/// Converts an x, y pair into a tilemap index
	pub fn xy_to_index(&self, x: i32, y: i32) -> usize {
		(y as usize * self.width as usize) + x as usize
	}
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
		self.tilemap[index] == TileType::Wall
	}
}

// EOF
