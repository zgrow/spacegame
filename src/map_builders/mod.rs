// mod.rs

use crate::map::*;
mod rexpaint_map;
use rexpaint_map::RexMapBuilder;
mod image_loader;

pub trait MapBuilder {
	fn build_map(&mut self);
	fn get_map(&self) -> Map;
}

pub fn random_builder(_new_depth: i32) -> Box<dyn MapBuilder>{
	//Box::new(SimpleMapBuilder::new())
	Box::new(RexMapBuilder::new())
}

// EOF
