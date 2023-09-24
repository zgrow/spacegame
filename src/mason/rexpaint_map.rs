// rexpaint_map.rs
// Loads a Rexpaint file into a Map object

use simplelog::*;
use bracket_rex::prelude::*;
use crate::mason::*;
use crate::engine::image_loader::load_rex_map;
use crate::components::Position;
use crate::artisan::ItemType;

pub struct RexMapBuilder {
	map: Map,
	new_entys: Vec<(ItemType, Position)>,
}

impl MapBuilder for RexMapBuilder {
	fn build_map(&mut self) {
		RexMapBuilder::load_test_map(self);
		debug!("* build_map::new_entys: {}", self.new_entys.len()); // DEBUG: announce creation of rexpaint map
	}
	fn get_map(&self) -> Map {
		self.map.clone()
	}
	fn get_item_spawn_list(&self) -> Vec<(ItemType, Position)> {
		info!("* dispatching new_entys"); // DEBUG:
		self.new_entys.clone()
	}
}

impl RexMapBuilder {
	pub fn new() -> RexMapBuilder {
		RexMapBuilder {
			map: Map::new(1, 1),
			new_entys: Vec::new(),
		}
	}
	fn load_test_map(&mut self) {
		(self.map, self.new_entys) = load_rex_map(&XpFile::from_resource("../resources/test_ship.xp").unwrap());
		debug!("* load_test_map::new_entys: {}", self.new_entys.len()); // DEBUG: announce loading the test map
	}
}

// EOF
