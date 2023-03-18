// components.rs

use bevy::prelude::*;
use tui::style::{Color, Modifier};

#[derive(Component, Resource)]
///Represents a point on a 2d grid as an xy pair
pub struct Position { pub x: i32, pub y: i32 }

#[derive(Component)]
///Makes the entity available to be rendered on the viewport
pub struct Renderable {
	// Field types selected for compat with tui::buffer::Cell
	glyph: String,  // stdlib
	fg: Color,      // tui-rs
	bg: Color,      // tui-rs
	mods: Modifier, // tui-rs
}

// EOF
