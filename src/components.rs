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
#[derive(Debug, Component, Reflect, Default, Clone, Copy)]
#[reflect(Component)]
pub struct LMR { }
#[derive(Debug, Component, Reflect, Default, Clone, Copy)]
#[reflect(Component)]
pub struct AURITA { }
/// Provides the player-facing identifier
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Name { pub name: String }
impl fmt::Display for Name {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.name)
	}
}
/// Represents a point on a 2d grid as an xy pair
#[derive(Reflect, Component, Resource, Copy, Clone, Eq, PartialEq, Default, Debug)]
#[reflect(Component)]
pub struct Position { pub x: i32, pub y: i32, pub z: i32 }
impl Position {
	pub fn new(new_x: i32, new_y: i32, new_z: i32) -> Position {
		Position{ x: new_x, y: new_y, z: new_z }
	}
	/// This is just a naive calculator for when all the variables can be obtained easily
	/// Thus it runs very quickly by virtue of not needing to call into the ECS
	/// Returns true if distance == range (ie is inclusive)
	pub fn in_range_of(&self, target: Position, range: i32) -> bool {
		if self.z != target.z { return false; } // z-levels must match (ie on same floor)
		if range == 0 {
			// This case is provided against errors; it's often faster/easier to just compare
			// positions directly in the situation where this method would be called
			if *self == target { return true; }
		} else {
			let mut d_x = f32::powi((target.y - self.y) as f32, 2);
			let mut d_y = f32::powi((target.x - self.x) as f32, 2);
			//eprintln!("dx: {}, dy: {}", d_x, d_y); // DEBUG:
			if d_x.signum() != 1.0 { d_x *= -1.0; }
			if d_y.signum() != 1.0 { d_y *= -1.0; }
			//eprintln!("dx: {}, dy: {}", d_x, d_y); // DEBUG:
			let distance = f32::sqrt(d_x + d_y).round();
			eprintln!("* calculated distance: {self:?} to {target:?}: {}", distance); // DEBUG:
			if distance as i32 <= range { return true; }
		}
		return false;
	}
}
impl fmt::Display for Position {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}, {}, {}", self.x, self.y, self.z)
	}
}
//impl<'a, 'b> PartialEq<&'b Position> for &'a Position {
//	fn eq(self, other: &'b Position) -> bool {
//		if self.x == other.x && self.y == other.y && self.z == other.z { true }
//		false
//	}
//}
impl<'a> PartialEq<&(i32, i32, i32)> for &'a Position {
	fn eq(&self, other: &&(i32, i32, i32)) -> bool {
		if self.x == other.0 && self.y == other.1 && self.z == other.2 { return true; }
		else { return false; }
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
#[derive(Reflect, Component)]
#[reflect(Component)]
pub struct Portable { pub carrier: Entity }
impl FromWorld for Portable {
	fn from_world(_world: &mut World) -> Self {
		Self {
			carrier: Entity::PLACEHOLDER,
		}
	}
}
/// Describes an entity that blocks line of sight; comes with an internal state for temp use
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Opaque { pub opaque: bool }
/// Describes an entity with an operable barrier of some kind: a container's lid, or a door, &c
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Openable {
	pub is_open: bool,
	pub open_glyph: String,
	pub closed_glyph: String,
}
/// Describes an entity that can operate lids/doors/&c
#[derive(Component)]
pub struct CanOpen { }
/// Describes an entity with something concealed behind a lock; uses an i32 value as a keyval
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Lockable { pub is_locked: bool, pub key: i32 }
/// Describes an entity which may contain entities tagged with the Portable Component
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Container { pub contents: Vec<String> }

//  *** PRIMITIVES AND COMPUTED VALUES (ie no save/load)
/// Sets the current run mode of the GameEngine
#[derive(Resource, Copy, Clone, Eq, PartialEq, Default, Debug, Reflect, FromReflect)]
#[reflect(Resource)]
pub enum EngineMode {
	#[default]
	Offline,
	Standby,    // ie when showing the startup menu, victory/game over screens, &c
	Startup,
	Running,
	Paused,
	GoodEnd,
	BadEnd,     // TODO: set up variants for both this and GoodEnd? maybe just a GameOver mode?
}
/// A convenient type that makes it clear whether we mean the Player entity or some other
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Creature {
	Player,     // The player(s)
	Zilch,      // Any non-player entity or character
}
/// The compass rose - note this is not a component...
/// These are mapped to cardinals just for ease of comprehension
//  On a boat, the directions are:
//  BOW:       front of the boat; fore == towards front
//  STERN:     rear of the boat; aft == towards back
//  STARBOARD: right side of the boat, facing fwd
//  PORT:      left side of the boat, facing fwd
//  ABOVE:     up above deck
//  BELOW:     down below deck
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
impl fmt::Display for Direction {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let text: String;
		match self {
			Direction::N    => { text = "north".to_string(); }
			Direction::NW   => { text = "northwest".to_string(); }
			Direction::W    => { text = "west".to_string(); }
			Direction::SW   => { text = "southwest".to_string(); }
			Direction::S    => { text = "south".to_string(); }
			Direction::SE   => { text = "southeast".to_string(); }
			Direction::E    => { text = "east".to_string(); }
			Direction::NE   => { text = "northeast".to_string(); }
			Direction::UP   => { text = "up".to_string(); }
			Direction::DOWN => { text = "down".to_string(); }
		}
		write!(f, "{}", text)
	}
}
/// Provides an object abstraction for the sensory range of a given entity
#[derive(Component)]
pub struct Viewshed {
	pub visible_tiles: Vec<Point>, //bracket_lib::pathfinding::field_of_view
	pub range: i32,
	pub dirty: bool, // indicates whether this viewshed needs to be updated from world data
}

//  *** GAME EVENTS
/// Provides the descriptors for GameEvents
/// Unless otherwise noted, any relevant event info will be included as a GameEventContext
/// TODO: optimize this to break up the events into different classes/groups so that the event
/// readers in the various subsystems only have to worry about their specific class of events
#[derive(Copy, Clone, Eq, PartialEq, Default)]
pub enum GameEventType {
	#[default]
	NullEvent,
	PauseToggle, // specifically causes a mode switch between Running <-> Paused
	ModeSwitch(EngineMode), // switches the engine to the specified mode
	PlayerMove(Direction),
	ActorOpen,
	ActorClose,
	ItemUse,
	ItemMove,
	ItemDrop,
	ItemKILL,
	DoorOpen,
	DoorClose,
	PlanqEvent(PlanqEventType),
}
impl fmt::Display for GameEventType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let output;
		match self {
			GameEventType::NullEvent => { output = "etype::NullEvent" }
			GameEventType::PauseToggle => { output = "etype::PauseToggle" }
			GameEventType::ModeSwitch(_) => { output = "etype::ModeSwitch" }
			GameEventType::PlayerMove(_) => { output = "etype::PlayerMove" }
			GameEventType::ActorOpen => { output = "etype::ActorOpen" }
			GameEventType::ActorClose => { output = "etype::ActorClose" }
			GameEventType::ItemUse => { output = "etype::ItemUse" }
			GameEventType::ItemMove => { output = "etype::ItemMove" }
			GameEventType::ItemDrop => { output = "etype::ItemDrop" }
			GameEventType::ItemKILL => { output = "etype::ItemKILL" }
			GameEventType::DoorOpen => { output = "etype::DoorOpen" }
			GameEventType::DoorClose => { output = "etype::DoorClose" }
			GameEventType::PlanqEvent(_) => { output = "etype::PlanqEvent" }
		}
		write!(f, "{}", output)
	}
}
/// Custom interface obj for passing data from ratatui to Bevy
#[derive(Resource, Default)]
pub struct GameEvent {
	pub etype: GameEventType,
	pub context: Option<GameEventContext>,
}
impl GameEvent {
	pub fn new(new_type: GameEventType, new_context: Option<GameEventContext>) -> GameEvent {
		GameEvent {
			etype: new_type,
			context: new_context,
		}
	}
}

/// Friendly bucket for holding contextual information about game actions
/// Note that this expresses a 1:1 relation: this preserves the atomic nature of the event
/// If an event occurs with multiple objects, then that event should be broken into multiple
#[derive(Resource)]
pub struct GameEventContext {
	pub subject: Entity, // the entity performing the action; by defn, only one
	pub object: Entity, // the entity upon which the subject will perform the action
}
/// Defines the set of control and input events that the Planq needs to handle
#[derive(Copy, Clone, Eq, PartialEq, Debug, Resource)]
pub enum PlanqEventType {
	Startup,
	Shutdown,
	Reboot,
	InventoryUse,
	InventoryDrop,
}

// EOF
