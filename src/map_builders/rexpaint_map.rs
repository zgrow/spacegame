// rexpaint_map.rs
// Loads a Rexpaint file into a Map object

use crate::map_builders::{
	*,
	image_loader::load_rex_map,
};
use bracket_rex::prelude::*;

pub struct RexMapBuilder {
	map: Map,
}

impl MapBuilder for RexMapBuilder {
	fn build_map(&mut self) {
		RexMapBuilder::load_test_map(self);
	}
	fn get_map(&self) -> Map {
		self.map.clone()
	}
}

impl RexMapBuilder {
	pub fn new() -> RexMapBuilder {
		RexMapBuilder {
			map: Map::new(1, 1, 1)
		}
	}
	fn load_test_map(&mut self) {
		self.map = load_rex_map(1, &XpFile::from_resource("../resources/test_ship.xp").unwrap());
	}
}

// EOF
