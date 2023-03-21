// components.rs

use crate::map::*;
use bevy::prelude::*;
use ratatui::style::{Color, Modifier};

///Represents a point on a 2d grid as an xy pair
#[derive(Component, Resource)]
pub struct Position { pub x: i32, pub y: i32 }

///Makes the entity available to be rendered on the viewport
#[derive(Component)]
pub struct Renderable {
	// Field types selected for compat with tui::buffer::Cell
	pub glyph: String,  // stdlib
	pub fg: Color,      // tui-rs
	pub bg: Color,      // tui-rs
	pub mods: Modifier, // tui-rs
}
///Represents a 'flattened' view of the Map's layers, with all entities and effects painted in,
///such that it can be read by the Viewport object when it comes time to render the view
#[derive(Resource)]
pub struct CameraView {
	pub map: Vec<Tile>,
	pub width: i32,
	pub height: i32,
}

// EOF
