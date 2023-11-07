// logical_map.rs
// November 6 2023

use simplelog::*;
use bevy::utils::hashbrown::HashMap;
use bevy::prelude::{
	Reflect,
	ReflectResource,
	Resource
};
use crate::mason::*;

//   - LOGICAL SHIP TOPOLOGY
// These linked-list directed graph routines were transcribed from:
// https://smallcultfollowing.com/babysteps/blog/2015/04/06/modeling-graphs-in-rust-using-vector-indices/
// on October 12, 2023
// Each Room (aka node) carries an index to its first _outgoing_ Door (aka edge), if present
// This first Door can optionally contain the indices of all the other Doors available to this Room
// Each Door contains an index to its destination Room, and the list of its companion Doors
// This way, a 1:1 relation is enforced between Rooms (nodes) and Doors (edges), modelling a linked list
// But actual rooms and doors in-game can have an x:y relation, or even an implicit, which models the game world
/// Simple enum wrappers to provide some type guarantees for these classes
pub type RoomIndex = usize; // An index to a GraphRoom
pub type DoorIndex = usize; // An index to a GraphDoor
/// Describes the entire logical map of the gameworld for map generation purposes
#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct ShipGraph {
	pub rooms: Vec<GraphRoom>,
	pub doors: Vec<GraphDoor>,
}
impl ShipGraph {
	/// Connects two GraphRooms with a GraphDoor
	pub fn connect(&mut self, go_from: RoomIndex, go_to: RoomIndex) {
		let door_index = self.doors.len();
		let room_data = &mut self.rooms[go_from];
		self.doors.push(GraphDoor {
			target: go_to,
			next_outgoing_door: room_data.first_outgoing_door,
			..GraphDoor::default()
		}); // the other values not defined above will be defaults
		room_data.first_outgoing_door = Some(door_index);
	}
	/// Adds a new GraphRoom to the ShipGraph
	pub fn add_room(&mut self, new_room: GraphRoom) -> RoomIndex {
		let index = self.rooms.len();
		self.rooms.push(new_room);
		index
	}
	/// Provides a recursive iterator that traverses the ShipGraph by links
	pub fn successors(&self, source: RoomIndex) -> Successors {
		let first_outgoing_door = self.rooms[source].first_outgoing_door;
		Successors { graph: self, current_door_index: first_outgoing_door }
	}
	/// Tests whether the specified Room is listed in the layout
	pub fn contains(&self, target: String) -> Option<usize> {
		for (index, room) in self.rooms.iter().enumerate() {
			if room.name == target {
				return Some(index);
			}
		}
		None
	}
	/// Gets the name of the Room that contains the given Position
	pub fn get_room_name(&self, target: Position) -> Option<String> {
		for room in &self.rooms {
			if room.contains(target) {
				return Some(room.name.clone());
			}
		}
		None
	}
	/// Adds a door to a room's logical map; returns False if it could not be added
	pub fn add_door_to_map_at(&mut self, mut target: Position) -> bool {
		// Find the index of the room that contains this position
		if let Some(room_index)  = self.rooms.iter().position(|x| x.contains(target)) {
			// Draw a line of Margin tiles from the door to the centerpoint of the room
			let centerpoint = self.rooms[room_index].centerpoint;
			if target.x != centerpoint.x && target.y != centerpoint.y {
				if target.x < centerpoint.x {
					target.x -= 1;
				} else {
					target.x += 1;
				}
			}
			let pathway = crate::mason::get_line(&centerpoint, &target);
			debug!("* {}", pathway.len());
			for point in pathway {
				self.rooms[room_index].new_interior.insert(point, GraphCell::new(CellType::Margin));
			}
			true
		} else {
			false
		}
	}
	/// Adds a stairway to a room's logical map, returns False if it could not be added
	pub fn add_stairs_to_map_at(&mut self, target: Position) -> bool {
		if let Some(room_index) = self.rooms.iter().position(|x| x.contains(target)) {
			// We want the stairs itself, and at least one Margin cell nearby depending on Walls
			// First mark the stairs itself as Closed
			self.rooms[room_index].new_interior.insert(target, GraphCell::new(CellType::Closed));
			// Then make a list of points to mark with Margin if they're Open
			let point_list = vec![
				Position::new(target.x + 1, target.y, target.z),
				Position::new(target.x - 1, target.y, target.z),
				Position::new(target.x, target.y + 1, target.z),
				Position::new(target.x, target.y - 1, target.z),
			];
			for point in point_list {
				if self.rooms[room_index].new_interior[&point].cell_type == CellType::Open {
					self.rooms[room_index].new_interior.insert(point, GraphCell::new(CellType::Margin));
				}
			}
			true
		} else {
			false
		}
	}
}

/// Describes a node in the topology graph, a single Room which is composed of a set of Positions
// A GraphRoom's interior positions will always be unique to itself: the same tile cannot be defined twice
// Some GraphRooms may share certain tiles, like walls, but the interiors will *always* be disjoint
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct GraphRoom {
	pub name: String,
	interior: Vec<Position>,
	first_outgoing_door: Option<RoomIndex>,
	// *** my new properties
	pub new_interior: HashMap<Position, GraphCell>,
	pub centerpoint: Position, // We prefer centerpoint over corner so that we can discern relative spatial locations
	pub ul_corner: Position,
	pub dr_corner: Position,
}
impl Default for GraphRoom {
	fn default() -> GraphRoom {
		GraphRoom {
			name: "blank_room".to_string(),
			interior: Vec::new(),
			first_outgoing_door: None,
			new_interior: HashMap::new(),
			centerpoint: Position::INVALID,
			ul_corner: Position::INVALID,
			dr_corner: Position::INVALID,
		}
	}
}
// JsonRoom:
// name: String
// exits: Vec<String> (other room names)
// corner: Vec<usize> (raw position triplet)
// width: usize (add +1 to include the right wall)
// height: usize (add +1 to include the lower wall)
// POSNS:
// centerpoint: corner.x + width / 2, corner.y + height / 2
// Wall UL: corner
// Wall DL: corner.x, corner.y + height
// Wall UR: corner.x + width, corner.y
// Wall DR: corner.x + width, corner.y + height
impl From<JsonRoom> for GraphRoom {
	fn from(new_room: JsonRoom) -> Self {
		// *** OLD
		let mut point_list: Vec<Position> = Vec::new();
		for whye in new_room.corner[1]..new_room.corner[1] + new_room.height {
			for echs in new_room.corner[0]..new_room.corner[0] + new_room.width {
				point_list.push(Position::new(echs as i32, whye as i32, new_room.corner[2] as i32));
			}
		}
		// END OLD
		// *** NEW
		let mut new_map: HashMap<Position, GraphCell> = HashMap::new();
		let z_level = new_room.corner[2];
		let full_width = new_room.width;
		let full_height = new_room.height;
		let ul_wall = (new_room.corner[0], new_room.corner[1]);
		let ul_inter = (ul_wall.0 + 1, ul_wall.1 + 1);
		let dr_wall = (ul_wall.0 + full_width, ul_wall.1 + full_height);
		let dr_inter = (dr_wall.0 - 1, dr_wall.1 - 1);
		let center = (ul_wall.0 + (full_width / 2), ul_wall.1 + (full_height / 2), z_level);
		// Populate the walls
		for wall_x in ul_wall.0..=(dr_wall.0) {
			new_map.insert((wall_x, ul_wall.1, z_level).into(), GraphCell::new(CellType::Blocked));
			new_map.insert((wall_x, dr_wall.1, z_level).into(), GraphCell::new(CellType::Blocked));
		}
		for wall_y in ul_wall.1..=(dr_wall.1) {
			new_map.insert((ul_wall.0, wall_y, z_level).into(), GraphCell::new(CellType::Blocked));
			new_map.insert((dr_wall.0, wall_y, z_level).into(), GraphCell::new(CellType::Blocked));
		}
		// Populate the interior tiles
		for inter_y in ul_inter.1..=dr_inter.1 {
			for inter_x in ul_inter.0..=dr_inter.0 {
				new_map.insert((inter_x, inter_y, z_level).into(), GraphCell::new(CellType::Open));
			}
		}
		// END NEW
		GraphRoom {
			name: new_room.name.clone(),
			interior: point_list,
			first_outgoing_door: None,
			new_interior: new_map,
			centerpoint: center.into(),
			ul_corner: (ul_wall.0, ul_wall.1, z_level).into(),
			dr_corner: (dr_wall.0, dr_wall.1, z_level).into(),
		}
	}
}
impl GraphRoom {
	/// Returns True if the specified Position is within the walls of the called Room
	pub fn contains(&self, target: Position) -> bool {
		//self.interior.contains(&target) || self.new_interior.contains_key(&target)
		self.new_interior.contains_key(&target)
	}
	pub fn set_interior_to(&mut self, new_interior: Vec<Position>) {
		self.interior = new_interior;
	}
	pub fn debug_print(&self) {
		let z_level = self.ul_corner.z;
		for whye in self.ul_corner.y..=(self.dr_corner.y) {
			let mut line_string: String = "".to_string();
			for echs in self.ul_corner.x..=(self.dr_corner.x) {
				let posn: Position = (echs, whye, z_level).into();
				if !self.new_interior.contains_key(&posn) {
					continue;
				}
				match self.new_interior[&posn].cell_type {
					CellType::Open => {
						line_string += ".";
					}
					CellType::Closed => {
						line_string += "O";
					}
					CellType::Margin => {
						line_string += "+";
					}
					CellType::Blocked => {
						line_string += "#";
					}
				}
			}
			debug!("{}", line_string);
		}
	}
}
/// Describes an edge in the topology graph, a connection between two GraphRooms
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct GraphDoor {
	pub name: String,
	pub from: Position,
	pub to: Position,
	target: RoomIndex,
	next_outgoing_door: Option<DoorIndex>,
}
impl Default for GraphDoor {
	fn default() -> GraphDoor {
		GraphDoor {
			name: "blank_door".to_string(),
			from: Position::default(),
			to: Position::default(),
			target: 0,
			next_outgoing_door: None,
		}
	}
}
/// Describes a single position in the topology graph, ie the smallest unit of space in the map
#[derive(Resource, Clone, Copy, Debug, Default, Reflect)]
pub struct GraphCell {
	pub cell_type: CellType,
}
impl GraphCell {
	pub fn new(new_type: CellType) -> GraphCell {
		GraphCell {
			cell_type: new_type,
		}
	}
}
/// Describes the different types of GraphCells in the map, which determine layout constraints
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum CellType {
	#[default]
	Open, // A Cell that can but does not have an occupant
	Closed, // A Cell that is occupied by something, such as a wall or an entity
	Blocked, // A Cell that is blocked by something terrain-ish, like a Wall
	Margin, // A Cell that must remain Open, ie cannot have an occupant
}

/// Simple iterator-ish object class for the ShipGraph
#[derive(Resource, Clone, Debug, Reflect)]
pub struct Successors<'a> {
	graph: &'a ShipGraph,
	current_door_index: Option<DoorIndex>,
}
impl<'a> Iterator for Successors<'a> {
	type Item = RoomIndex;
	fn next(&mut self) -> Option<RoomIndex> {
		match self.current_door_index {
			None => None,
			Some(door_num) => {
				let door = &self.graph.doors[door_num];
				self.current_door_index = door.next_outgoing_door;
				Some(door.target)
			}
		}
	}
}


// EOF
