// components.rs
// Contains the definitions for assorted small ECS component structs

use bevy::prelude::*;
use bracket_pathfinding::prelude::*;

/// *** SAVE/LOAD ELIGIBLE
/// Provides a "tag" component for the Player entity, easy retrieval
#[derive(Debug, Component, Reflect, Default, Clone, Copy)]
#[reflect(Component)]
pub struct Player { }
//  TODO: later going to add a LMR, AURITA tag...
/// Effectively a unique ID for an entity
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Name { pub name: String }
/// Represents a point on a 2d grid as an xy pair
#[derive(Reflect, Component, Resource, Copy, Clone, Eq, PartialEq, Default, Debug)]
#[reflect(Component)]
pub struct Position { pub x: i32, pub y: i32, pub z: i32 }
/// Makes the entity available to be rendered on the viewport
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Renderable {
	// Field types selected for compat with tui::buffer::Cell
	pub glyph: String,  // stdlib
	pub fg: u8,      // tui-rs as a Color::Indexed
	pub bg: u8,      // tui-rs
	//pub mods: Modifier, // tui-rs
}
/// Describes an entity that can move around, and includes an index to their associated floor/depth
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Mobile { }
/// Describes an entity that obstructs movement by other entities
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Blocking { }

//  *** PRIMITIVES AND COMPUTED VALUES (ie no save/load)
/// The compass rose - note this is not a component...
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Direction {
	N,
	NW,
	W,
	SW,
	S,
	SE,
	E,
	NE,
	UP,
	DOWN
}
/// Provides the descriptors for TUIEvent
pub enum GameEventType {
	PlayerMove(Direction),
}
/// Custom interface obj for passing data from ratatui to Bevy
#[derive(Resource)]
pub struct TuiEvent {
	pub etype: GameEventType,
}
/// Provides an object abstraction for the sensory range of a given entity
#[derive(Component)]
pub struct Viewshed {
	pub visible_tiles: Vec<Point>, //bracket_lib::pathfinding::field_of_view
	pub range: i32,
	pub dirty: bool, // indicates whether this viewshed needs to be updated from world data
}
// EOF
