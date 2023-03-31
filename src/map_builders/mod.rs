// mod.rs
// Provides the heavy lifting for building maps without cluttering up main()

use crate::map::*;
mod rexpaint_map;
use rexpaint_map::RexMapBuilder;

pub trait MapBuilder {
	fn build_map(&mut self);
	fn get_map(&self) -> Map;
}

pub fn random_builder(_new_depth: i32) -> Box<dyn MapBuilder>{
	Box::new(RexMapBuilder::new())
}

// EOF
