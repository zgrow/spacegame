// components.rs
// July 12 2023
// ###: COMPONENTS REFERENCE LIST
/* components.rs
 *   AccessPort - "accessport"
 *   ActionSet - "actionset"
 *     actions: HashSet<ActionType>
 *     outdated: bool
 *   Body - "body NNN"
 *     ref_posn: Position
 *     extent: Vec<Glyph>
 *   Container - "container"
 *   Description - "description name desc"
 *     name: String
 *     desc: String
 *     locn: String (set during gameplay, specify its Body.ref_posn instead)
 *   Device - "device state voltage discharge"
 *     pw_switch: bool
 *     batt_voltage: i32
 *     batt_discharge: i32
 *     state: DeviceState (gameplay property)
 *   Glyph - use a Body component for this instead
 *     posn: Position
 *     cell: ScreenCell
 *   IsCarried - "iscarried"
 *   Key - "key id"
 *     key_id: i32
 *   LMR - "lmr"
 *   Lockable - "lockable state key_id"
 *     is_locked: bool
 *     key_id: i32
 *   Memory - "memory"
 *     visual: HashMap<Position, Vec<Entity>>
 *   Mobile - "mobile"
 *   Networkable - "networkable"
 *   Obstructive - "obstructive"
 *   Opaque - "opaque state"
 *     opaque: bool
 *   Openable - "openable state stuck open closed"
 *     is_open: bool
 *     is_stuck: bool
 *     open_glyph: String
 *     closed_glyph: String
 *   Player - "player"
 *   Portable - "portable"
 *     carrier: Entity
 *   Viewshed - "viewshed range"
 *     visible_tiles: Vec<Point>
 *     range: i32
 *     dirty: bool
 */
/* camera.rs
 *   CameraView
 *     output: Vec<ScreenCell>
 *     width: i32
 *     height: i32
 *     reticle: Position
 *     reticle_glyphs: String
 *   ScreenCell
 *     glyph: String
 *     fg: u8
 *     bg: u8
 *     modifier: u16
 */
/* planq/mod.rs
 *   Planq - "planq"
 *   PlanqProcess - "planqprocess"
 *     timer: Timer
 *     outcome: PlanqEvent
 */
/* planq/monitor.rs
 *   DataSampleTimer - "datasampletimer"
 *     timer: Timer
 *     source: String
 */

// ###: EXTERNAL LIBS
use std::fmt;
use std::hash::Hash;
use bevy::prelude::{
	Component,
	FromWorld,
	Reflect,
	ReflectComponent,
	ReflectResource,
	Resource,
	World,
};
use bevy::ecs::entity::*;
use bevy::utils::hashbrown::{HashMap, HashSet};
use bracket_pathfinding::prelude::*;
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;
//use simplelog::*;

// ###: INTERNAL LIBS
use crate::engine::event::ActionType;
use crate::camera::ScreenCell;

// Full-length derive macro examples
//#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
//#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]

//   ##: ActionSet
/// Allows an entity to identify the set of ActionTypes that it supports.
/// The presence of an ActionType in actions indicates it is compatible;
/// finding the intersection between two ActionSets results in the set of actions
/// that one entity may execute on another
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct ActionSet {
	#[reflect(ignore)]
	pub actions: HashSet<ActionType>,
	#[reflect(ignore)]
	pub outdated: bool,
}
impl ActionSet {
	pub fn new() -> Self {
		ActionSet::default()
	}
}
impl Default for ActionSet {
	fn default() -> ActionSet {
		ActionSet {
			actions: HashSet::new(),
			outdated: true,
		}
	}
}
//   ##: Description
/// Holds the narrative description of an object. If this component is used as an input for text formatting, it will produce
/// the name of the entity that owns it. See also the name() and desc() methods
#[derive(Component, Clone, Debug, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Description {
	pub name: String, // The 'friendly' name of the Entity
	pub desc: String, // The long-form description of the Entity
	pub locn: String, // The name of the room that this Entity occupies
}
impl Description {
	/// Creates a new Description with the given name and description
	pub fn new() -> Description {
		Description::default()
	}
	pub fn name(mut self, new_name: &str) -> Self {
		self.name = new_name.to_string();
		self
	}
	pub fn desc(mut self, new_desc: &str) -> Self {
		self.desc = new_desc.to_string();
		self
	}
	pub fn locn(mut self, new_locn: &str) -> Self {
		self.locn = new_locn.to_string();
		self
	}
	pub fn get_name(&self) -> String {
		self.name.clone()
	}
	pub fn get_desc(&self) -> String {
		self.desc.clone()
	}
	pub fn get_locn(&self) -> String {
		self.locn.clone()
	}
}
impl Default for Description {
	fn default() -> Description {
		Description {
			name: "default_name".to_string(),
			desc: "default_desc".to_string(),
			locn: "default_locn".to_string(),
		}
	}
}
impl fmt::Display for Description {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.name)
	}
}
//   ##: Body
/// Defines the shape/form of an Entity's physical body within the gameworld, defined on absolute game Positions
/// Allows Entities to track all of their physical shape, not just their canonical Position
/// NOTE: if an Entity's 'extended' Body is supposed to use different glyphs, then the Renderable.glyph
/// property should be set to the _entire_ string, in order, that the game should render
/// ie the Positions listed in Body.extent need to correspond with the chars in the Entity's Renderable.glyph
/// If there aren't enough chars to cover all the given Positions, then the last-used char will be repeated
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct Body { // aka Exterior, Veneer, Mass, Body, Visage, Shape, Bulk, Whole
	pub ref_posn: Position,
	pub extent: Vec<Glyph>,
}
impl Body {
	//    #: Builder
	pub fn new() -> Body {
		Body::default()
	}
	/// Creates a new Body component from a set of input strings, formatted as "x,y G F B M" where 'x,y' or 'x,y,z'
	/// is the spawnpoint coordinates; 'G' is the display glyph, 'F' is the foreground color, 'B' is the background
	/// color, and 'M' is the set of text modifications to apply to the display glyph
	pub fn new_from_str(input: Vec<String>) -> Body {
		//debug!("* recvd input: {:?}", input);
		if input.is_empty() { return Body::default(); };
		let mut posns = Vec::new();
		let mut cells = Vec::new();
		for line in input.iter() {
			let mut body_parts = line.split(' ');
			if let Some(posn) = body_parts.next() {
				posns.push(posn.into());
			}
			//cells.push(ScreenCell::new_from_str(&body_parts.collect::<Vec<&str>>().join(" "))); // HINT: rejoins back into string
			cells.push(ScreenCell::new_from_str_vec(body_parts.collect()));
		}
		Body::large(posns, cells)
	}
	/// Creates a new Body component for an single-tile-sized Entity
	pub fn small(new_posn: Position, new_glyph: ScreenCell) -> Body {
		Body {
			ref_posn: new_posn,
			extent: vec![(new_posn, new_glyph).into()]
		}
	}
	/// Creates a new Body component for a multitile Entity: if there are more Positions than ScreenCells given,
	/// then the remaining Positions will be filled with copies of the last ScreenCell in the list;
	/// If there are more ScreenCells than Positions, the remainder will be silently dropped
	pub fn large(posns: Vec<Position>, mut glyphs: Vec<ScreenCell>) -> Body {
		// Pad out the list of glyphs if it's not long enough
		loop {
			if posns.len() <= glyphs.len() {
				break;
			}
			if let Some(last_glyph) = glyphs.last() {
				glyphs.push(last_glyph.clone());
			}
		}
		// Assign the first Position in the list as the reference position, and then make the full extent of the new Body
		Body {
			ref_posn: posns[0],
			extent: posns.iter().zip(glyphs.iter()).map(|x| x.into()).collect(),
		}
	}
	//    #: Get/Set
	/// Returns true if any of this Body's parts are occupying the given Position
	pub fn contains(&self, target: &Position) -> bool {
		for piece in self.extent.iter() {
			if piece.posn == *target {
				return true;
			}
		}
		false
	}
	/// Returns true if any of this Body's parts are within a single tile of the target
	pub fn is_adjacent_to(&self, target: &Position) -> bool {
		self.in_range_of(target, 1)
	}
	/// Performs a simple/naive range check between this Body component and the given Position; will return true
	/// if _any_ of the Body's parts are within range
	pub fn in_range_of(&self, target: &Position, range: i32) -> bool {
		for point in self.extent.iter() {
			if point.posn.in_range_of(target, range) {
				return true;
			}
		}
		false
	}
	/// Moves this Body's parts to the new Position without losing cohesion
	pub fn move_to(&mut self, target: Position) {
		let posn_diff = target - self.ref_posn;
		for glyph in self.extent.iter_mut() {
			glyph.posn += posn_diff;
		}
		//debug!("move_to: {}({:?}) to {} => {:?}", self.ref_posn, self.extent, target, posn_diff);
		self.ref_posn = target;
	}
	/// Produces the set of Positions that this Body would occupy if it moved to the target Position
	pub fn project_to(&self, target: Position) -> Vec<Position> {
		let mut posn_list = Vec::new();
		let posn_diff = target - self.ref_posn;
		for glyph in self.extent.iter() {
			let new_posn = glyph.posn + posn_diff;
			posn_list.push(new_posn);
		}
		posn_list
	}
	/// Returns the full set of Positions that this Body currently occupies
	pub fn posns(&self) -> Vec<Position> {
		self.extent.iter().map(|x| x.posn).collect()
	}
	/// Retrieves a particular glyph at a particular position; returns None if nothing found
	pub fn glyph_at(&self, target: &Position) -> Option<Glyph> {
		self.extent.iter().find(|x| x.posn == *target).cloned()
	}
	/// Sets a particular Glyph at a particular Position of a given Entity; returns false if the change failed for
	/// one reason or another, such as an invalid Position
	pub fn set_glyph_at(&mut self, target: Position, glyph: &str) -> bool {
		if let Some(index) = self.extent.iter().position(|x| x.posn == target) {
			self.extent[index].cell.set_glyph(glyph);
			true
		} else {
			false
		}
	}
	/// (possible deprecation!) Sets a Body's extent to the given list of Glyphs
	#[deprecated]
	pub fn glyphs(mut self, new_glyphs: Vec<Glyph>) -> Self {
		self.extent = new_glyphs;
		self
	}
}
//    #: Glyph
/// Represents a single ScreenCell as a part of an Entity; the Body component can use more than one of these
/// to construct a multitile entity
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct Glyph {
	pub posn: Position,
	pub cell: ScreenCell,
}
impl Glyph {
	pub fn new() -> Glyph {
		Glyph::default()
	}
	pub fn posn(mut self, target: Position) -> Self {
		self.posn = target;
		self
	}
	pub fn cell(mut self, new_cell: ScreenCell) -> Self {
		self.cell = new_cell;
		self
	}
}
impl From<(Position, ScreenCell)> for Glyph {
	fn from(value: (Position, ScreenCell)) -> Self {
		Glyph {
			posn: value.0,
			cell: value.1,
		}
	}
}
impl From<(&Position, &ScreenCell)> for Glyph {
	fn from(value: (&Position, &ScreenCell)) -> Self {
		Glyph {
			posn: *value.0,
			cell: value.1.clone(),
		}
	}
}
impl From<Glyph> for ScreenCell {
	fn from(value: Glyph) -> ScreenCell {
		value.cell
	}
}
//   ##: Viewshed
/// Provides an object abstraction for the sensory range of a given entity
//  INFO: This Viewshed type is NOT eligible for bevy_save because bracket_lib::Point doesn't impl Reflect/FromReflect
#[derive(Component, Clone, Debug)]
pub struct Viewshed {
	pub visible_points: Vec<Point>, // for bracket_lib::pathfinding::field_of_view
	pub range: i32,
	pub dirty: bool, // indicates whether this viewshed needs to be updated from world data
	// TODO: Adding an Entity type to the enty_memory ought to allow for retrieving that information later, so that the
	// player's own memory can be queried, something like the Nethack dungeon feature notes tracker
}
impl Viewshed {
	pub fn new(new_range: i32) -> Self {
		Self {
			visible_points: Vec::new(),
			range: new_range,
			dirty: true,
		}
	}
}
//    ##: Memory
/// Provides a memory of seen entities and other things to an entity with sentience
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct Memory {
	#[reflect(ignore)]
	pub visual: HashMap<Position, Vec<Entity>>,
}
impl Memory {
	pub fn new() -> Self {
		Memory::default()
	}
	/// Updates the memorized positions for the specified entity; adds to memory if not already present; clears the memory
	/// if there's nothing there any more
	pub fn update(&mut self, targets: Vec<(Position, Option<Vec<Entity>>)>) {
		for (posn, entys) in targets.iter() {
			if let Some(guys) = entys {
				self.visual.insert(*posn, guys.clone());
			} else {
				self.visual.remove(posn);
			}
		}
	}
}
//   ##: Portable
/// Describes an entity that can be picked up and carried around
//#[derive(Component, Clone, Copy, Debug, Default)]
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct Portable {
	pub carrier: Entity
}
impl Portable {
	pub fn new(target: Entity) -> Portable { Portable { carrier: target } }
	pub fn empty() -> Portable { Portable { carrier: Entity::PLACEHOLDER } }
}
impl MapEntities for Portable {
	fn map_entities(&mut self, entity_mapper: &mut EntityMapper) {
		self.carrier = entity_mapper.get_or_reserve(self.carrier);
	}
}
impl FromWorld for Portable {
	// This is intentional (lmao) to prevent issues when loading from save game
	fn from_world(_world: &mut World) -> Self {
		Self {
			carrier: Entity::PLACEHOLDER,
		}
	}
}
//   ##: Opaque
/// Describes an entity that blocks line of sight; comes with an internal state for temp use
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Opaque {
	pub opaque: bool
}
impl Opaque {
	pub fn new(setting: bool) -> Self {
		Opaque {
			opaque: setting,
		}
	}
}
//   ##: Openable
/// Describes an entity with an operable barrier of some kind: a container's lid, or a door, &c
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Openable {
	pub is_open: bool,
	pub is_stuck: bool,
	pub open_glyph: String,
	pub closed_glyph: String,
}
impl Openable {
	pub fn new(state: bool, opened: &str, closed: &str) -> Openable {
		Openable {
			is_open: state,
			is_stuck: false,
			open_glyph: opened.to_string(),
			closed_glyph: closed.to_string(),
		}
	}
}
//   ##: Lockable
/// Describes an Entity that can be locked and unlocked, such as a door or a locker
// FIXME: how does this prevent something from being unlocked from the 'wrong' side?
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Lockable {
	pub is_locked: bool,
	pub key_id: i32
}
impl Lockable {
	// Unlocks, given the correct key value as input
	pub fn unlock(&mut self, test_key: i32) -> bool {
		if test_key == self.key_id {
			self.is_locked = false;
			return true;
		}
		false
	}
	// Locks when called; if a key is given, it will overwrite the previous key-value
	// Specify a value of 0 to obtain the existing key-value instead
	pub fn lock(&mut self, new_key: i32) -> i32 {
		self.is_locked = true;
		if new_key != 0 { self.key_id = new_key; }
		self.key_id
	}
}
//   ##: Key
/// Describes an entity that can lock or unlock a Lockable object
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Key { pub key_id: i32 }
//   ##: Device
/// Describes an entity with behavior that can be applied/used/manipulated by another entity
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
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
		self.pw_switch = !self.pw_switch;
		self.pw_switch
	}
}
//    #: DeviceState
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum DeviceState {
	#[default]
	Offline,
	Idle,
	Working,
	Error(u32) // Takes an error code as a specifier
}

//  ###: TAG COMPONENTS
//   ##: Player
/// Identifies the Entity that represents the player character
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Player { }
//   ##: LMR
/// Identifies the LMR in the ECS
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct LMR { }
//   ##: IsCarried
/// Describes an Entity that is currently located within a Container
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct IsCarried { }
//   ##: Container
/// Describes an entity which may contain entities tagged with the Portable Component
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Container { } // TODO: this almost definitely needs a capacity field attached to it
//   ##: AccessPort
/// Describes an entity with a PLANQ-compatible maintenance system
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct AccessPort { }
//   ##: Networkable
/// Describes an entity that can connect to and communicate with the shipnet
#[derive(Component, Copy, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct Networkable { }
//   ##: Mobile
/// Describes an Entity that can move around under its own power
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Mobile { }
//   ##: Obstructive
/// Describes an entity that obstructs movement by other entities
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Obstructive { }

//  ###: PRIMITIVES AND COMPUTED VALUES (ie no save/load)
//   ##: Color
/// A small type that lets us specify friendly names for colors instead of using ints everywhere
/// Because none of these carry any data, and because Rust implicitly zero-indexes them, they can be cast
/// directly to numeric types using the "as" keyword: "my_color_var as u8"
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum Color {
	// These are arranged in order of their ANSI index
	Black,    // 00
	Red,      // 01
	Green,    // 02
	Yellow,   // 03
	Blue,     // 04
	Pink,     // 05
	Cyan,     // 06
	White,    // 07
	#[default]
	LtBlack,  // 08
	LtRed,    // 09
	LtGreen,  // 10
	LtYellow, // 11
	LtBlue,   // 12
	LtPink,   // 13
	LtCyan,   // 14
	LtWhite   // 15
}
//   ##: Direction
/// The compass rose - note this is not a component...
/// These are mapped to cardinals just for ease of comprehension
#[derive(AsRefStr, Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component)]
pub enum Direction {
	#[default]
	X,
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
			Direction::X    => { "null_dir".to_string() }
			Direction::N    => { "North".to_string() }
			Direction::NW   => { "Northwest".to_string() }
			Direction::W    => { "West".to_string() }
			Direction::SW   => { "Southwest".to_string() }
			Direction::S    => { "South".to_string() }
			Direction::SE   => { "Southeast".to_string() }
			Direction::E    => { "East".to_string() }
			Direction::NE   => { "Northeast".to_string() }
			Direction::UP   => { "Up".to_string() }
			Direction::DOWN => { "Down".to_string() }
		};
		write!(f, "{}", text)
	}
}
//   ##: Position
/// Represents a point on a 2D grid as an XY pair, plus a Z-coordinate to indicate what floor the entity is on
#[derive(Component, Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[reflect(Component, Resource)]
pub struct Position {
	pub x: i32,
	pub y: i32,
	pub z: i32,
}
impl Position {
	/// A handy constant for checking if a map coordinate is invalid
	pub const INVALID: Position = Position{x: -1, y: -1, z: -1};
	/// Creates a new Position from the given values
	pub fn new(new_x: i32, new_y: i32, new_z: i32) -> Position {
		Position{ x: new_x, y: new_y, z: new_z }
	}
	/// This is just a naive calculator for when all the variables can be obtained easily
	/// Thus it runs very quickly by virtue of not needing to call into the ECS
	/// Returns true if distance == range (ie is inclusive)
	pub fn in_range_of(&self, target: &Position, range: i32) -> bool {
		//debug!("* Testing range {} between positions {} to {}", range, self, target); // DEBUG: announce range check
		if self.z != target.z { return false; } // z-levels must match (ie on same floor)
		if range == 0 {
			// This case is provided against errors; it's often faster/easier to just compare
			// positions directly in the situation where this method would be called
			if self == target { return true; }
		} else {
			let mut d_x = f32::powi((target.y - self.y) as f32, 2);
			let mut d_y = f32::powi((target.x - self.x) as f32, 2);
			//debug!("dx: {}, dy: {}", d_x, d_y); // DEBUG: print the raw values for dx, dy
			if d_x.signum() != 1.0 { d_x *= -1.0; }
			if d_y.signum() != 1.0 { d_y *= -1.0; }
			//debug!("dx: {}, dy: {}", d_x, d_y); // DEBUG: print the normalized values for dx, dy
			let distance = f32::sqrt(d_x + d_y).round();
			//debug!("* in_range_of(): calc dist = {self:?} to {target:?}: {} in range {} -> {}", distance, range, (distance as i32 <= range)); // DEBUG: print the result of the calculation
			if distance as i32 <= range { return true; }
		}
		false
	}
	/// Checks if two Positions are next to each other; shorthand for calling `self.in_range_of(target, 1)`
	pub fn is_adjacent_to(&self, target: &Position) -> bool {
		self.in_range_of(target, 1)
	}
	/// Converts map coordinates to screen coordinates
	/// WARN: this method does NOT guarantee or validate the coordinates it generates; if a given Position
	/// would fall offscreen, then that is what will be returned!
	/// The player's position is required as the second parameter in order to provide a reference point between the two maps
	pub fn to_camera_coords(&self, screen: Rect, p_map: Position) -> Position {
		// We can discard the z coordinate, since we can only see one level at a time anyway
		// We can also assume the following relation/analogy: centerpoint : screen :: p_map : worldmap
		let c_x = screen.width / 2;
		let c_y = screen.height / 2;
		let d_x = p_map.x - self.x;
		let d_y = p_map.y - self.y;
		Position::new(c_x as i32 - d_x, c_y as i32 - d_y, 0)
	}
	/// A special method that produces the difference between the two Positions as integers,
	/// intended for use in index-based loops to allow simple iteration
	pub fn difference(&self, rhs: &Position) -> (i32, i32, i32) {
		((rhs.x - self.x), (rhs.y - self.y), (rhs.z - self.z))
	}
	/// Returns true if the Position doesn't have any negative parts
	pub fn is_valid(&self) -> bool {
		if self.x < 0 { return false; }
		if self.y < 0 { return false; }
		if self.z < 0 { return false; }
		true
	}
}
impl From<&str> for Position {
	/// Parses a comma-separated string into a Position triplet; will return the Position::INVALID if there are problems
	/// parsing the input; if no z-coordinate is specified, will be set to zero
	fn from(input: &str) -> Self {
		let parts: Vec<&str> = input.split(',').collect();
		if parts.len() < 2 || parts.len() > 3 {
			return Position::INVALID;
		}
		let echs = parts[0].parse::<i32>().unwrap_or(-1);
		let whye = parts[1].parse::<i32>().unwrap_or(-1);
		let zhee = if parts.len() < 3 { 0 } else { parts[2].parse::<i32>().unwrap_or(-1) };
		Position {
			x: echs,
			y: whye,
			z: zhee
		}
	}
}
impl From<(i32, i32, i32)> for Position {
	fn from(value: (i32, i32, i32)) -> Self {
		Position {
			x: value.0,
			y: value.1,
			z: value.2,
		}
	}
}
impl From<(usize, usize, usize)> for Position {
	fn from(value: (usize, usize, usize)) -> Self {
		Position{
			x: value.0 as i32,
			y: value.1 as i32,
			z: value.2 as i32,
		}
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
impl std::ops::Add<(i32, i32, i32)> for Position {
	type Output = Position;
	fn add(self, rhs: (i32, i32, i32)) -> Position {
		Position {
			x: self.x + rhs.0,
			y: self.y + rhs.1,
			z: self.z + rhs.2,
		}
	}
}
impl std::ops::AddAssign<(i32, i32, i32)> for Position {
	fn add_assign(&mut self, rhs: (i32, i32, i32)) {
		self.x += rhs.0;
		self.y += rhs.1;
		self.z += rhs.2;
	}
}
impl std::ops::Add<PosnOffset> for Glyph {
	type Output = Glyph;
	fn add(self, rhs: PosnOffset) -> Glyph {
		Glyph {
			posn: self.posn + rhs,
			cell: self.cell
		}
	}
}
//    #: PosnOffset
/// Provides some ergonomics around Rust's type handling so that there's less "x as usize" casting everywhere;
/// used for small adjustments on a grid map in the SAME z-level; if a z-level transition is required look elsewhere
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PosnOffset {
	pub x_diff: i32,
	pub y_diff: i32,
	pub z_diff: i32,
}
impl PosnOffset {
	pub fn new(echs: i32, whye: i32, zhee: i32) -> PosnOffset {
		PosnOffset {
			x_diff: echs,
			y_diff: whye,
			z_diff: zhee,
		}
	}
}
impl std::ops::Add<PosnOffset> for Position {
	type Output = Position;
	fn add(self, rhs: PosnOffset) -> Position {
		Position {
			x: self.x + rhs.x_diff,
			y: self.y + rhs.y_diff,
			z: self.z + rhs.z_diff,
		}
	}
}
impl std::ops::AddAssign<PosnOffset> for Position {
	fn add_assign(&mut self, rhs: PosnOffset) {
		*self = *self + rhs;
	}
}
impl std::ops::Sub<Position> for Position {
	type Output = PosnOffset;
	fn sub(self, rhs: Position) -> PosnOffset {
		PosnOffset {
			x_diff: self.x - rhs.x,
			y_diff: self.y - rhs.y,
			z_diff: self.z - rhs.z,
		}
	}
}
/* NOTE: Defn for "Position - PosnOffset = Position" is disabled due to uncertainty; subtraction on a PosnOffset
 *       that contains negative values will almost definitely produce unexpected behavior...
 *	impl std::ops::Sub<PosnOffset> for Position {
 *	type Output = Position;
 *	fn sub(self, rhs: PosnOffset) -> Position {
 *		Position {
 *			x: self.x - rhs.x_diff,
 *			y: self.y - rhs.y_diff,
 *			z: self.z - rhs.z_diff,
 *		}
 *	}
 *}
 *impl std::ops::SubAssign<PosnOffset> for Position {
 *	fn sub_assign(&mut self, rhs: PosnOffset) {
 *		*self = *self - rhs;
 *	}
 *}
*/
/* NOTE: Defn for "Position + Position = Position" is disabled due to uncertainty:
 * vector sums are useful when trying to calculate the amount of force applied to a body,
 * but that isn't useful right now since I have no physics to worry about
*/

// EOF
