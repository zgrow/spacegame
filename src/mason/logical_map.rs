// logical_map.rs
// November 6 2023

use simplelog::*;
use bevy::utils::hashbrown::HashMap;
use bevy::prelude::{
	Reflect,
	ReflectResource,
	Resource
};
use bevy_turborand::GlobalRng;
use bevy_turborand::DelegatedRng;
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
	/// Gets the RoomIndex of the named Room
	pub fn get_room_index(&self, target: &str) -> Option<RoomIndex> {
		self.rooms.iter().position(|x| x.name == target)
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
			for point in pathway {
				self.rooms[room_index].new_interior.insert(point, CellType::Margin);
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
			self.rooms[room_index].new_interior.insert(target, CellType::Closed);
			// Then make a list of points to mark with Margin if they're Open
			let point_list = vec![
				Position::new(target.x + 1, target.y, target.z),
				Position::new(target.x - 1, target.y, target.z),
				Position::new(target.x, target.y + 1, target.z),
				Position::new(target.x, target.y - 1, target.z),
			];
			for point in point_list {
				if self.rooms[room_index].new_interior[&point] == CellType::Open {
					self.rooms[room_index].new_interior.insert(point, CellType::Margin);
				}
			}
			true
		} else {
			false
		}
	}
	/// Returns the list of all rooms currently listed in the internal graph
	pub fn get_room_list(&self) -> Vec<String> {
		self.rooms.iter().map(|x| x.name.clone()).collect()
	}
}

/// Describes how to place an item in a room
#[derive(Clone, Debug, Default)]
pub struct SpawnTemplate {
	pub shape: Vec<(Qpoint, CellType, bool)>, // Represents the whole template including margins and blocked spaces
	output: Vec<(String, String, (usize, usize))>, // (id, name, (x, y)) - Represents only the occupied spaces
	constraints: Option<Vec<(String, String)>>
}
impl SpawnTemplate {
	pub fn new() -> SpawnTemplate {
		SpawnTemplate::default()
	}
	pub fn is_successful(&self) -> bool {
		// I'm disabling the lint here because I want it to be *explicit* that the filter is ONLY doing a comparison
		// This is because I already tried to write this as a filter_map and the compiler lost its damn mind
		#![allow(clippy::bool_comparison)]
		let trues: Vec<bool> = self.shape.iter().filter(|x| x.2 == true).map(|x| x.2).collect();
		trues.len() == self.shape.len()
	}
	pub fn reset_success(&mut self) {
		// Rust claims that this doesn't work because the state variable is never read from? WTF????????
		//for (_, _, mut state) in self.shape.iter_mut() {
		//	state = false;
		//}
		for slot in self.shape.iter_mut() {
			slot.2 = false;
		}
	}
	/// Generates a set of real-world Positions from a given reference Position using the template's shape
	pub fn realize_coordinates(&self, ref_posn: &Position) -> Vec<(String, Position)> {
		let mut output = Vec::new();
		for (_id, name, posn) in &self.output {
			let next_point: Position = Position {
				x: ref_posn.x + posn.0 as i32,
				y: ref_posn.y + posn.1 as i32,
				z: ref_posn.z
			};
			output.push((name.clone(), next_point));
		}
		//debug!("* Generated a Position set for ref_posn {:?}", ref_posn); // DEBUG: log the generated Position set
		output
	}
	/// Simple helper for putting constraint rules into a new SpawnTemplate
	pub fn add_constraints(&mut self, new_rules: Vec<(String, String)>) {
		self.constraints = Some(new_rules.clone());
	}
	/// Replaces the IDs in a SpawnTemplate with a single string; usually meant for single-item templates, but note that
	/// this will work just the same on a template with multiple entity positions!
	pub fn assign_name(&mut self, name: String) {
		for item in self.output.iter_mut() {
			item.1 = name.clone();
		}
	}
	/// Replaces the IDs in a SpawnTemplate with the item names in a RawItemSet's contents list
	pub fn assign_names(&mut self, name_list: Vec<(String, String)>) {
		// name_list's values are (id, name) as per defn from furniture_groups_v1.json
		// NOTE: you can't get mutable refs out of a Rust vector unless it was created that way
		// so trying to get mutable refs into a tuple binding will always fail unless the vector was
		// initialized with, eg, vec![&String] and NOT vec![String]
		// For every occupied tile in the template,
		//eprintln!("* recvd name_list: {:?}", name_list); // DEBUG: log received name_list
		for item in self.output.iter_mut() {
			if let Some(new_name) = name_list.iter().find(|x| x.0 == item.0) { // If it matches a tile in the defn,
				item.1 = new_name.1.clone(); // Assign it a real name
			}
		}
	}
}
impl From<Vec<Vec<String>>> for SpawnTemplate {
	fn from(_input: Vec<Vec<String>>) -> SpawnTemplate {
		//eprintln!("* From<Vec<Vec<String>> input: {:?}", input); // DEBUG: log the received input
		todo!("! SpawnTemplate::From<Vec<Vec<String>>> still unimplemented");
		//SpawnTemplate::new()
	}
}
impl From<Vec<String>> for SpawnTemplate {
	fn from(input: Vec<String>) -> SpawnTemplate {
		//eprintln!("* From<Vec<String>> input: {:?}", input); // DEBUG: log the received input
		let mut new_output = Vec::new();
		let mut new_shape = Vec::new();
		for (height, line) in input.iter().enumerate() {
			for (width, chara) in line.chars().enumerate() {
				let new_type = match chara {
					'.' => { /* free space, just skip the position */ continue; },
					'+' => { CellType::Margin },
					'#' => { CellType::Wall },
					'A'..='Z' => { // This position will be occupied, add it to the spawn list
						new_output.push((chara.to_string(), "spawn_template_default_name".to_string(), (width, height)));
						CellType::Closed
					}
					 _  => { error!("* Unrecognized celltype character: {}", chara); CellType::Wall }
				};
				new_shape.push(((width as f32, height as f32), new_type, false));
			}
		}
		SpawnTemplate {
			shape: new_shape.clone(),
			output: new_output.clone(),
			constraints: None
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
	pub new_interior: HashMap<Position, CellType>,
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
		let mut new_map: HashMap<Position, CellType> = HashMap::new();
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
			new_map.insert((wall_x, ul_wall.1, z_level).into(), CellType::Wall);
			new_map.insert((wall_x, dr_wall.1, z_level).into(), CellType::Wall);
		}
		for wall_y in ul_wall.1..=(dr_wall.1) {
			new_map.insert((ul_wall.0, wall_y, z_level).into(), CellType::Wall);
			new_map.insert((dr_wall.0, wall_y, z_level).into(), CellType::Wall);
		}
		// Populate the interior tiles
		for inter_y in ul_inter.1..=dr_inter.1 {
			for inter_x in ul_inter.0..=dr_inter.0 {
				new_map.insert((inter_x, inter_y, z_level).into(), CellType::Open);
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
		//debug!("--- interior map for GraphRoom {}", self.name);
		for whye in self.ul_corner.y..=(self.dr_corner.y) {
			let mut line_string: String = "".to_string();
			for echs in self.ul_corner.x..=(self.dr_corner.x) {
				let posn: Position = (echs, whye, z_level).into();
				if !self.new_interior.contains_key(&posn) {
					continue;
				}
				match self.new_interior[&posn] {
					CellType::Open => {
						line_string += ".";
					}
					CellType::Closed => {
						line_string += "O";
					}
					CellType::Margin => {
						line_string += "+";
					}
					CellType::Wall => {
						line_string += "#";
					}
				}
			}
			//debug!("{}", line_string); // DEBUG: prints the collision detection map of the room
		}
	}
	/// Locates an open space to spawn an item given its associated SpawnTemplate; if successful,
	/// returns the set of occupied Positions and the SpawnTemplate IDs that correspond to them
	pub fn find_open_space(&mut self, mut template: SpawnTemplate, rng: &mut GlobalRng) -> Option<Vec<(String, Position)>> {
		// METHOD
		// given the template as input, and the destination as the target,
		// choose a random point in the destination to start at
		// iterate through the template, matching cell types
		// if at any point there is a failure to match, try a new point
		// repeat until either a valid starting point is found,
		// or all starting points are exhausted
		let possible_starts: Vec<Position> = self.new_interior.iter() // All points in the interior of the room...
			.filter(|x| *x.1 == template.shape[0].1 || *x.1 == CellType::Open) // ...which have the same CellType as the template's first point, or are Open...
			.map(|x| *x.0).collect(); // ...mapped into a Vec<Position> and gathered up
		if possible_starts.is_empty() { return None; } // Early return if there were no candidate points at all
		// start with a list of all points that match the type of the template's ref_point
		// choose a point in the list randomly
		//for s_point in rng.sample_iter(possible_starts.iter()) {
		while let Some(ref_point) = rng.sample_iter(possible_starts.iter()) {
			// TODO: ->> "choose from one of a set of loaded template shapes"
			for (t_point, t_type, t_success) in template.shape.iter_mut() {
				// Derive the next Position to examine
				let next_point: Position = Position {
					x: ref_point.x + t_point.0 as i32,
					y: ref_point.y + t_point.1 as i32,
					z: ref_point.z
				};
				// If the derived point isn't even in the bounds of the room, try the next
				if !self.new_interior.contains_key(&next_point) {
					//debug!("* Tested point is not within room bounds, trying new ref_point...");
					template.reset_success();
					break;
				}
				// Examine the destination cell's type to see if placing the template there is legal
				// This has to be done case-by-case because the rules for which types can change are a bit complex
				// TODO: strongly consider removing this logic to its own method
				match self.new_interior[&next_point] {
					CellType::Open   => { // An Open cell can be set to Closed or Margin but not Wall
						if *t_type != CellType::Wall { *t_success = true; }
					}
					CellType::Closed => { // A Closed cell is considered wholly occupied and cannot accept anything
						// Do nothing
					}
					CellType::Wall   => { // A Wall cell always matches with Walls but not other types
						if *t_type == CellType::Wall { *t_success = true; }
					}
					CellType::Margin => { // A Margin cell can be placed on an Open or an existing Margin cell
						if *t_type == CellType::Open || *t_type == CellType::Margin { *t_success = true; }
					}
				}
				//debug!("* Tested {:?} vs {:?} @{:?}: {}", t_type, self.new_interior[&next_point], next_point, t_success);
			}
			// Checks the success state of each tile in the template to make sure it was placeable
			if template.is_successful() {
				// Update the room's interior layout map to contain the newly placed object
				self.update_interior(&template, ref_point);
				//return Some(template.into_positions(s_point)); // DEBUG: using longer method below for debugging info
				let final_item_list = template.realize_coordinates(ref_point);
				//debug!("* --> Found valid template posn set: {:?}", final_item_list); // DEBUG: log template success
				return Some(final_item_list);
			}
			// At least one of the template's points failed, reset the template and the output list for another try
			//debug!("* Could not find valid open space, trying new ref_point..."); // DEBUG: log template failure
			template.reset_success();
		}
		None // Should only occur here if all possible starts were tried with no success
	}
	pub fn update_interior(&mut self, template: &SpawnTemplate, ref_point: &Position) {
		for t_point in template.shape.iter() {
			let next_point: Position = Position {
				x: ref_point.x + t_point.0.0 as i32,
				y: ref_point.y + t_point.0.1 as i32,
				z: ref_point.z
			};
			self.new_interior.insert(next_point, t_point.1);
		}
		self.debug_print();
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
	Closed, // A Cell that is occupied by something like an entity
	Wall, // A Cell that is blocked by something terrain-ish, like a Wall
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
