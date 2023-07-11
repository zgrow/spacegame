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
#[derive(Reflect, FromReflect, Component, Resource, Copy, Clone, Eq, PartialEq, Default, Debug)]
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
			//eprintln!("* calculated distance: {self:?} to {target:?}: {}", distance); // DEBUG:
			if distance as i32 <= range { return true; }
		}
		false
	}
}
impl PartialEq<(i32, i32, i32)> for Position {
	fn eq(&self, rhs: &(i32, i32, i32)) -> bool {
		if self.x == rhs.0 && self.y == rhs.1 && self.z == rhs.2 { return true; }
		false
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
/// Describes an entity which may contain entities tagged with the Portable Component
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Container { pub contents: Vec<String> }
/// Describes an entity that blocks line of sight; comes with an internal state for temp use
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Opaque { pub opaque: bool }
/// Describes an entity with an operable barrier of some kind: a container's lid, or a door, &c
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Openable {
	pub is_open: bool,
	pub is_stuck: bool,
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
impl Lockable {
	// Unlocks, given the correct key value as input
	pub fn unlock(&mut self, test_key: i32) -> bool {
		if test_key == self.key {
			self.is_locked = false;
			return true;
		}
		false
	}
	// Locks when called; if a key is given, it will overwrite the previous key-value
	// Specify a value of 0 to obtain the existing key-value instead
	pub fn lock(&mut self, new_key: i32) -> i32 {
		self.is_locked = true;
		if new_key != 0 { self.key = new_key; }
		self.key
	}
}
/// Describes an entity that can lock or unlock a Lockable object
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Key { pub key_id: i32 }
/// Describes an entity that can have an external power source, a switch, and a DeviceState
#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Device {
	pub pw_switch: bool,
	pub batt_voltage: i32,
	pub batt_discharge: i32,
	pub state: DeviceState,
}
impl Device {
	/// Creates a new Device; set the batt_discharge param to 0 to disable battery use
	pub fn new(discharge_rate: i32) -> Device {
		Device {
			pw_switch: false,
			batt_voltage: 0, // BATTERIES NOT INCLUDED LMAOOO
			batt_discharge: discharge_rate,
			state: DeviceState::Offline,
		}
	}
	/// Turns on the device, if there's any power remaining. Returns false if no power left.
	pub fn power_on(&mut self) -> bool {
		if self.batt_voltage > 0
		|| self.batt_discharge == 0 {
			self.pw_switch = true;
			self.state = DeviceState::Idle;
		}
		self.pw_switch
	}
	/// Turns off the device.
	pub fn power_off(&mut self) {
		self.pw_switch = false;
		self.state = DeviceState::Offline;
	}
	/// Discharges battery power according to the specified duration, returns current power level
	pub fn discharge(&mut self, duration: i32) -> i32 {
		if self.batt_discharge < 0 {
			// This item does not need a battery/has infinite power, so no discharge can occur
			return self.batt_voltage;
		}
		self.batt_voltage -= self.batt_discharge * duration;
		if self.batt_voltage < 0 { self.batt_voltage = 0; }
		self.batt_voltage
	}
	/// Recharges the battery to the given percentage
	pub fn recharge(&mut self, charge_level: i32) -> i32 {
		self.batt_voltage += charge_level;
		self.batt_voltage
	}
	/// power toggle
	pub fn power_toggle(&mut self) -> bool {
		// NOTE: trying to invoke these methods doesn't seem to work here; not sure why
		//if !self.pw_switch { self.power_on(); }
		//else { self.power_off(); }
		if !self.pw_switch { self.pw_switch = true; }
		else { self.pw_switch = false; }
		self.pw_switch
	}
}
#[derive(Reflect, Default, Copy, Clone, Eq, PartialEq)]
pub enum DeviceState {
	#[default]
	Offline,
	Idle,
	Working,
	Error(u32) // Takes an error code as a specifier
}
/// Describes an entity that can manipulate the controls of a Device
#[derive(Component)]
pub struct CanOperate { }
/// Describes an entity with a PLANQ-compatible maintenance system
#[derive(Component)]
pub struct AccessPort { }
/// Describes an entity that can connect to and communicate with the shipnet
#[derive(Reflect, FromReflect, Default, Copy, Clone, Eq, PartialEq, Debug)]
pub struct Networkable { }

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
		let text: String = match self {
			Direction::N    => { "north".to_string() }
			Direction::NW   => { "northwest".to_string() }
			Direction::W    => { "west".to_string() }
			Direction::SW   => { "southwest".to_string() }
			Direction::S    => { "south".to_string() }
			Direction::SE   => { "southeast".to_string() }
			Direction::E    => { "east".to_string() }
			Direction::NE   => { "northeast".to_string() }
			Direction::UP   => { "up".to_string() }
			Direction::DOWN => { "down".to_string() }
		};
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

// EOF
