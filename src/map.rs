// map.rs
use bracket_algorithm_traits::prelude::{Algorithm2D, BaseMap};
use bracket_geometry::prelude::*;
//use specs::prelude::*;
//use specs_derive::*;
//use super::*;

pub const MAPWIDTH: i32 = 80;
pub const MAPHEIGHT: i32 = 60;
pub const MAPSIZE: i32 = MAPWIDTH * MAPHEIGHT;

pub fn xy_to_index(x: i32, y: i32, width: i32) -> usize {
	(y as usize * width as usize) + x as usize
}
#[derive(PartialEq, Copy, Clone, Debug, Default)]
pub enum TileType {
	#[default]
	Floor,
	Wall,
}
#[derive(Clone, Debug)]
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
			revealed_tiles: vec![false; (new_width * new_height) as usize],
			visible_tiles: vec![false; (new_width * new_height) as usize],
		}
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
