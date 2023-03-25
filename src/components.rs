// components.rs
// Contains the definitions for assorted small ECS component structs

use crate::map::*;
use bevy::prelude::*;
use ratatui::style::{Color, Modifier};

///Provides a "tag" component for the Player entity, easy retrieval
#[derive(Debug, Component, Default, Clone, Copy)]
pub struct Player { }
// TODO: later going to add a LMR, AURITA tag...
///Effectively a unique ID for an entity
#[derive(Component)]
pub struct Name { pub name: String }
///Represents a point on a 2d grid as an xy pair
#[derive(Component, Resource, Copy, Clone)]
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
///Describes an entity that can move around
#[derive(Component)]
pub struct Mobile { }
#[derive(Copy, Clone, Eq, PartialEq)]
///The compass rose - note this is not a component...
pub enum Direction {
	N,
	NW,
	W,
	SW,
	S,
	SE,
	E,
	NE
}
///Custom interface obj for passing data from ratatui to Bevy
#[derive(Resource)]
pub struct TuiEvent {
	pub etype: GameEvent,
}
///Provides the descriptors for TUIEvent
pub enum GameEvent {
	PlayerMove(Direction),
}


// EOF
