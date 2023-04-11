// components.rs
// Contains the definitions for assorted small ECS component structs

use bevy::prelude::*;
use bracket_pathfinding::prelude::*;
use std::fmt;

//  *** SAVE/LOAD ELIGIBLE
/// Provides a "tag" component for the Player entity, easy retrieval
#[derive(Debug, Component, Reflect, Default, Clone, Copy)]
#[reflect(Component)]
pub struct Player { }
/// Tag component for the player's PLANQ
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Planq { }
//  TODO: later going to add a LMR, AURITA tag...
/// Effectively a unique ID for an entity
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Name { pub name: String }
/// Represents a point on a 2d grid as an xy pair
#[derive(Reflect, Component, Resource, Copy, Clone, Eq, PartialEq, Default, Debug)]
#[reflect(Component)]
pub struct Position { pub x: i32, pub y: i32, pub z: i32 }
impl Position {
	pub fn new(new_x: i32, new_y: i32, new_z: i32) -> Position {
		Position{ x: new_x, y: new_y, z: new_z }
	}
}
impl fmt::Display for Position {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}, {}, {}", self.x, self.y, self.z)
	}
}
/// Makes the entity available to be rendered on the viewport
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Renderable {
	// Field types selected for compat with tui::buffer::Cell
	pub glyph: String,  // stdlib
	pub fg: u8,      // tui-rs as a Color::Indexed
	pub bg: u8,      // tui-rs
	//pub mods: Modifier, // tui-rs
	//pub priority: i32, // stdlib: TODO: determines whether this entity is drawn over other entities
}
/// Describes an entity that can move around, and includes an index to their associated floor/depth
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Mobile { }
/// Describes an entity that obstructs movement by other entities
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Obstructive { }
/// Describes an entity that can be picked up and carried around
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Portable { }
/// Describes an entity with an operable barrier of some kind: a container's lid, or a door, &c
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Openable { pub is_open: bool }
/// Describes an entity with something concealed behind a lock; uses an i32 value as a keyval
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Lockable { pub is_locked: bool, pub key: i32 }
/// Describes an entity which may contain entities tagged with the Portable Component
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Container { pub contents: Vec<String> }

//  *** PRIMITIVES AND COMPUTED VALUES (ie no save/load)
/// A convenient type that makes it clear whether we mean the Player entity or some other
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Creature {
	Player,     // The player(s)
	Zilch,      // Any non-player entity or character
}
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
	ItemPickup(Creature),
	//ItemDrop(Position)?
	//ItemGive(???)?
}
/// Custom interface obj for passing data from ratatui to Bevy
#[derive(Resource)]
pub struct GameEvent {
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
