// artisan/mod.rs
// Describes the class of inanimate objects throughout the game, both Props and Furniture

/* Notes on JSON-to-Rust-struct mapping:
 * A struct defn like so:
 *   pub struct RawItem {
 *     pub description: Vec<String>,
 *     pub body: Vec<String>,
 *     pub placement: String,
 *     pub components: Option<ComponentBundle>, // *see note below
 *   }
 * Will require a JSON definition like so:
 *   "rawitem": [
 *     {
 *       "description": ["name", "long description"],
 *       "body": ["@", "fg_color", "bg_color", "mods"],
 *       "placement": "O+",
 *       "components": {
 *         "accessport": "",
 *         "openable": ["O", "C"],
 *         "device": 128
 *       }
 *    }
 *  ]
 * *A defn for the ComponentBundle type is left as an exercise for the reader
 * Note also that *any* JSON dictionary becomes a struct with named fields in Rust
 * If the field names *cannot* be exact matches, use serde macros to handle renaming
 */
/* The full list of components as of Nov 8 2023:
 * REQUIRED:
 *   Body, Description
 *   - The Position component is a part of the Body component, which is preferred for game entities
 *   - The Renderable component is also a part of the Body component
 *   - The Description component includes the entity's name
 * TAGS:
 *   AccessPort
 *   ActionSet
 *   Container
 *   IsCarried
 *   Memory
 *   Mobile
 *   Networkable
 *   Obstructive
 * COMPLEX:
 *   Device(discharge rate in volts/turn as i32)
 *   Key(key id as i32)
 *   Lockable(initial state as bool, matching key id as i32)
 *   Opaque(current state as bool)
 *   Openable(initial state as bool, open/closed glyphs)
 *   Portable(carrier of item as Entity)
 *   Viewshed(range in tiles as i32)
 */

// CLIPPY SHUT UPPPPPPPPPP
#![allow(unused_variables)]
#![allow(dead_code)]

// ###: EXTERNAL LIBRARIES
use simplelog::*;
use std::fs::File;
use std::io::BufReader;
use serde::{Deserialize, Serialize};
use bevy::prelude::{
	Bundle,
	Entity,
	Reflect,
	ReflectResource,
	Resource,
	World,
};
use bevy::ecs::world::EntityMut;
use bevy_turborand::*;

// ###: INTERNAL LIBRARIES
use crate::components::*;
use crate::engine::planq::*;
use crate::worldmap::*;
use furniture::Facade;
use crate::mason::logical_map::SpawnTemplate;

pub mod furniture;

//  ###: COMPLEX TYPES
//   ##: THE ITEM BUILDER
//    #: ItemBuilder
/// Provides a facility for creating items during gameplay
#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct ItemBuilder {
	request_list: Vec<ItemRequest>, // The template ID, the item name, ...
	pub spawn_count: i32,
	body:     Option<Body>,
	desc:     Option<Description>,
	actions:  Option<ActionSet>,
	// Optional/auxiliary components
	access:   Option<AccessPort>,
	contain:  Option<Container>,
	device:   Option<Device>,
	is_carried: Option<IsCarried>,
	key:      Option<Key>,
	lock:     Option<Lockable>,
	mobile:   Option<Mobile>,
	network:  Option<Networkable>,
	obstruct: Option<Obstructive>,
	opaque:   Option<Opaque>,
	open:     Option<Openable>,
	portable: Option<Portable>,
	planq:    Option<Planq>,
	backdrop: Option<Facade>,
	#[reflect(ignore)]
	item_dict:     ItemDict,
}
impl<'a, 'b> ItemBuilder where 'a: 'b {
	/// ItemBuilder constructor
	pub fn new() -> ItemBuilder {
		// -- NEW METHOD
		// Load the item definitions from the external files
		// Parse the raw item data into local structures
		// Return the new object instance
		// -- OLD METHOD
		ItemBuilder {
			item_dict: load_furniture_defns("resources/furniture_items_v3.json", "resources/furniture_sets_v2.json"),
			..ItemBuilder::default()
		}
	}
	/// Starting incantation in the chain to create new items
	pub fn create(&mut self, new_item: &str) -> &mut ItemBuilder {
		//debug!("* ItemBuilder create() request: {}", new_item); // DEBUG: log item builder request
		if let Some(item_data) = self.item_dict.furniture.iter().find(|x| x.name == new_item) {
			self.desc = Some(Description::new().name(&item_data.name).desc(&item_data.desc));
			//debug!("* recvd item_data.body: {:?}", item_data.body.clone()); // DEBUG: log new Body component
			self.body = Some(Body::new_from_str(item_data.body.clone()));
			if !item_data.extra.is_empty() {
				// Parse and add any additional components that are in the item's definition
				//debug!("* recvd item_data.extra: {:?}", item_data.extra); // DEBUG: log any extra components
				for component in item_data.extra.iter() {
					//debug!("* raw component value: {}", component); // DEBUG: log raw component values
					// HINT: This will in fact return the entire string if the string consists of only a single word
					//    let new_string: Vec<&str> = component.split(' ').collect();
					let mut new_cmpnt = component.split(' ');
					let part = new_cmpnt.next().unwrap();
					let details: Vec<&str> = new_cmpnt.collect();
					let error_msg = "! ERR: Could not parse key:value for ";
					match part {
						"accessport"  => { self.access = Some(AccessPort::default()); } // tag component
						"actionset"   => { self.actions = Some(ActionSet::default()); } // tag component
						"container"   => { self.contain = Some(Container::default()); } // tag component for now
						"description" => {
							let mut new_desc = Description::new();
							for string in details.iter() {
								if let Some((key, value)) = string.split_once(':') {
									match key {
										"name" => { new_desc.name = value.to_string(); }
										"desc" => { new_desc.desc = value.to_string(); }
										_ => { warn!("* component key:value {}:{} was not recognized", key, value); }
									}
								} else { warn!("* could not split key:value on component {}", part); }
							}
							self.desc = Some(new_desc);
						}
						"device"      => {
							let mut new_device = Device::new(0);
							for string in details.iter() {
								if let Some((key, value)) = string.split_once(':') {
									match key {
										"state" => { new_device.pw_switch = value.parse().expect(&(error_msg.to_owned() + "device:state")); }
										"voltage" => { new_device.batt_voltage = value.parse().expect(&(error_msg.to_owned() + "device:voltage")); }
										"rate" => { new_device.batt_discharge = value.parse().expect(&(error_msg.to_owned() + "device:rate")); }
										_ => { warn!("* component key:value {}:{} was not recognized", key, value); }
									}
								} else { warn!("* could not split key:value on component {}", part); }
							}
							self.device = Some(new_device);
						}
						"key"         => {
							let mut new_key = Key::default();
							for string in details.iter() {
								if let Some((key, value)) = string.split_once(':') {
									if key == "id" { new_key.key_id = value.parse().expect(&(error_msg.to_owned() + "key:id")); }
									else { warn!("* component key:value {}:{} was not recognized", key, value); }
								} else { warn!("* could not split key:value on component {}", part); }
							}
							self.key = Some(new_key);
						}
						"lockable"    => {
							let mut new_lock = Lockable::default();
							for string in details.iter() {
								if let Some((key, value)) = string.split_once(':') {
									match key {
										"state" => { new_lock.is_locked = value.parse().expect(&(error_msg.to_owned() + "lockable:state")); }
										"key_id" => { new_lock.key_id = value.parse().expect(&(error_msg.to_owned() + "lockable:key_id")); }
										_ => { warn!("* component key:value {}:{} was not recognized", key, value); }
									}
								} else { warn!("* could not split key:value on component {}", part); }
							}
							self.lock = Some(new_lock);
						}
						"mobile"      => { self.mobile = Some(Mobile::default()); } // tag component
						"networkable" => { self.network = Some(Networkable::default()); } // tag component
						"obstructs"   => { self.obstruct = Some(Obstructive::default()); } // tag component
						"opaque"      => {
							let mut new_opaque = Opaque::default();
							if details.is_empty() {
								// The default for a boolean in Rust is 'false', which means that the Opaque::default()
								// is an Opaque component with component.opaque = false, meaning transparent
								new_opaque.opaque = true;
							} else {
								for string in details.iter() {
									if let Some((key, value)) = string.split_once(':') {
										if key == "state" { new_opaque.opaque = value.parse().expect(&(error_msg.to_owned() + "opaque:state")); }
										else { warn!("* component key:value {}:{} was not recognized", key, value); }
									}
								}
							}
							self.opaque = Some(new_opaque);
						}
						"openable"    => {
							let mut new_open = Openable::default();
							for string in details.iter() {
								if let Some((key, value)) = string.split_once(':') {
									match key {
										"state" => { new_open.is_open = value.parse().expect(&(error_msg.to_owned() + "openable:state")); }
										"stuck" => { new_open.is_stuck = value.parse().expect(&(error_msg.to_owned() + "openable:stuck")); }
										"open" => { new_open.open_glyph = value.to_string(); }
										"closed" => { new_open.closed_glyph = value.to_string(); }
										_ => { warn!("* component key:value {}:{} was not recognized", key, value); }
									}
								} else { warn!("* could not split key:value on component {}", part); }
							}
							self.open = Some(new_open);
						}
						"portable"    => { self.portable = Some(Portable::empty()); } // the Entity field cannot be specified before runtime
						_ => { error!("! ERR: requested component {} was not recognized", component); }
					}
				}
			}
		}
		/*
		 * else if let Some(set_data) = self.item_dict.sets.iter().find(|x| x.name == new_item) {
		 * 	// There's no way to store the values for multiple items to be generated, so instead we'll make this
		 * 	// the method that gets things set up for the spawn call later
		 * 	debug!("* Setting up an item spawn batch request: {}", set_data.name); // DEBUG: 
		 * 	eprintln!("* Setting up an item spawn batch request: {}", set_data.name); // DEBUG:
		 * 	for request in set_data.contents.iter() {
		 * 		self.request_list.push(ItemRequest::new(request.0.clone(), request.1.clone()));
		 * 	}
		 * } else {
		 * 	eprintln!("! ERR: item request '{}' not found in dictionary!", new_item);
		 * 	error!("! ERR: item request '{}' not found in dictionary!", new_item); // FIXME: WHY AREN'T THESE WORKING
		 * }
		 */
		self
	}
	#[deprecated(note = "--> Switch to using ItemBuilder::create(str)")]
	pub fn create_by_itemtype(&mut self, new_type: ItemType) -> &mut ItemBuilder {
		match new_type {
			ItemType::Simple    => {
				self.desc = Some(Description::new().name(&format!("_simpleItem_{}", self.spawn_count)).desc("A simple Item."));
				self.actions = Some(ActionSet::new());
			}
			ItemType::Thing     => {
				self.desc = Some(Description::new().name(&format!("_thing_{}", self.spawn_count)).desc("A new Thing."));
				self.actions = Some(ActionSet::new());
				self.portable = Some(Portable::empty());
			}
			ItemType::Fixture   => {
				self.desc = Some(Description::new().name(&format!("_fixture_{}", self.spawn_count)).desc("A plain Fixture."));
				self.actions = Some(ActionSet::new());
				self.obstruct = Some(Obstructive::default());
				self.opaque = Some(Opaque::new(true));
			}
			ItemType::Furniture => {
				self.desc = Some(Description::new().name(&format!("_furnish_{}", self.spawn_count)).desc("A piece of Furniture."));
				self.actions = Some(ActionSet::new());
				self.obstruct = Some(Obstructive::default());
				self.opaque = Some(Opaque::new(true));
			}
			ItemType::Scenery   => {
				self.backdrop = Some(Facade::default());
				self.obstruct = Some(Obstructive::default());
				self.opaque = Some(Opaque::new(true));
			}
			ItemType::Door      => {
				self.desc = Some(Description::new().name(&format!("_door_{}", self.spawn_count)).desc("A regular Door."));
				self.actions = Some(ActionSet::new());
				self.obstruct = Some(Obstructive::default());
				self.opaque = Some(Opaque::new(true));
				self.open = Some(Openable::new(false, "▔", "█",));
			}
			ItemType::Snack     => {
				self.desc = Some(Description::new().name(&format!("_snack_{}", self.spawn_count)).desc("A tasty Snack."));
				self.actions = Some(ActionSet::new());
				self.portable = Some(Portable::empty());
			}
			ItemType::Planq     => {
				self.desc = Some(Description::new().name("PLANQ").desc("It's your PLANQ."));
				self.actions = Some(ActionSet::new());
				self.portable = Some(Portable::empty());
				self.device = Some(Device::new(-1));
				self.planq = Some(Planq::new());
			}
		}
		self
	}
	/// Sets the item's position in the game world, given the ref_point to spawn it at
	pub fn at(&mut self, posn: Position) -> &mut ItemBuilder {
		if self.request_list.is_empty() {
			if let Some(body) = self.body.as_mut() {
				body.move_to(posn);
			}
		} else {
			for item in self.request_list.iter_mut() {
				item.destination = Some(posn);
			}
		}
		self
	}
	/// Sets an item's position as being in an Entity's inventory
	pub fn give_to(&mut self, target: Entity) -> &mut ItemBuilder {
		if self.request_list.is_empty() {
			self.portable = Some(Portable::new(target));
			self.is_carried = Some(IsCarried::default());
		} else {
			for item in self.request_list.iter_mut() {
				item.recipient = Some(target);
			}
		}
		self
	}
	/// Constructs the item into the specified Bevy::App, and returns the generated Entity ID as well as the full set
	/// of Positions, aka the Body.extent, aka the item's shape, that the item occupies on the map
	pub fn build(&'b mut self, world: &'a mut World) -> Vec<(EntityMut<'b>, Vec<Position>)> {
		self.spawn_count += 1;
		let mut item_shape = Vec::new();
		let mut new_item = world.spawn_empty();
		// Add all of the populated components to the new entity
		if let Some(desc)     = &self.desc { new_item.insert(desc.clone()); self.desc = None; }
		if let Some(body)     = &self.body {
			//debug!("* creating new item with shape {:?}", body.posns());
			item_shape = body.posns();
			new_item.insert(body.clone()); self.body = None;
		}
		if let Some(actions)  = &self.actions { new_item.insert(actions.clone()); self.actions = None; }
		if let Some(backdrop) = self.backdrop { new_item.insert(backdrop); self.backdrop = None; }
		if let Some(contain)  = &self.contain { new_item.insert(*contain); self.contain = None; }
		if let Some(device)   = self.device { new_item.insert(device); self.device = None; }
		if let Some(is_carried) = self.is_carried { new_item.insert(is_carried); self.is_carried = None; }
		if let Some(key)      = self.key { new_item.insert(key); self.key = None; }
		if let Some(lock)     = self.lock { new_item.insert(lock); self.lock = None; }
		if let Some(mobile)   = self.mobile { new_item.insert(mobile); self.mobile = None; }
		if let Some(obstruct) = self.obstruct { new_item.insert(obstruct); self.obstruct = None; }
		if let Some(opaque)   = self.opaque { new_item.insert(opaque); self.opaque = None; }
		if let Some(open)     = &self.open { new_item.insert(open.clone()); self.open = None; }
		if let Some(planq)    = self.planq { new_item.insert(planq); self.planq = None; }
		if let Some(portable) = self.portable { new_item.insert(portable); self.portable = None; }
		vec![(new_item, item_shape)]
	}
	/// Generates the list of decorative items that the worldgen will need to spawn
	pub fn decorate(&mut self, worldmap: &WorldModel) -> Vec<Position> {
		// Each placement entry MUST provide:
		// - A defn for the item's Description component
		// - A defn for the item's Body component
		// Technically the ActionSet component is not required but leaving it out creates very boring objects
		//let posns = Vec::new();
		// Get the list of rooms we're going to decorate
		let room_names = worldmap.get_room_name_list();
		for name in room_names.iter() {
			// Get the room's list of candidate items
			// Each object defn must include:
			// - Description: name, description
			// - Body: at least one Position and ScreenCell pair
			// [the ActionSet component will be automatically attached except in special cases]
			// - An ItemPattern for the object, eg if it is furniture and needs some walkway margin
			// - Any other additional components to be included
			let max_count = 0; // ERROR: this will need to be assigned per-room later on
			//let candidates = self.get_candidates(name);
			// Generate placements for all mandatory objects
			let mut generated_count = 0;
			loop {
				// Try to generate an item
				// If successful, incremement generated_count
				if generated_count >= max_count {
					break;
				}
				generated_count += 1;
			}
			// Generate placements for any additional/optional objects
			
		}
		// - Return the entire list of placements
		//posns
		todo!("Not done implementing this yet"); // TODO: finish implementing the decorate() method in artisan
	}
	/// Retrieves a random template from the set defined for a specified item
	pub fn get_random_shape(&self, item_name: &str, rng: &mut GlobalRng) -> Option<SpawnTemplate> {
		//debug!("* get_random_shape: {}", item_name); // DEBUG: log get_random_shape invocation
		// If this item name was found in the ItemDict,
		if let Some(item_data) = self.item_dict.furniture.iter().find(|x| x.name == item_name) {
			// Return a SpawnTemplate that is made from the 'furniture' list of RawItems in the ItemDict
			// item_data should be a RawItem object, representing a single item, so it's okay to return wholesale
			//debug!("* Obtained item_data: {:?}", item_data); // DEBUG: log obtained item_data
			let mut new_template: SpawnTemplate = (*rng.sample(&item_data.shapes).unwrap()).clone().into();
			new_template.assign_name(item_data.name.clone());
			return Some(new_template);
		} else if let Some(set_data) = self.item_dict.sets.iter().find(|x| x.name == item_name) {
			// As above, but for the 'sets' list of RawItemSets in the ItemDict
			// Make a base template using the item set defn
			let mut new_template: SpawnTemplate = (*rng.sample(&set_data.shapes).unwrap()).clone().into();
			// Use the room's contents list from the item defn, to populate the names in the spawn template's output
			//debug!("* RNG: Now calling assign_names with {:?}", set_data.contents); // DEBUG: log obtained item_data
			new_template.assign_names(set_data.contents.clone());
			return Some(new_template);
		} else {
			// Couldn't find the requested item, make sure someone knows
			error!("! No entry for requested item '{}' in furniture_items or furniture_sets", item_name);
		}
		None
	}
}
//   ##: ItemRequest
#[derive(Resource, Clone, Debug, Default, Reflect)]
pub struct ItemRequest {
	pub placement: String,
	pub name: String,
	pub destination: Option<Position>,
	pub recipient: Option<Entity>,
}
impl ItemRequest {
	pub fn new(new_id: String, new_name: String) -> ItemRequest {
		ItemRequest {
			placement: new_id,
			name: new_name,
			destination: None,
			recipient: None,
		}
	}
}
//    #: ItemData
/// Passing this data structure to an ItemBuilder will take care of the entire item creation request
/// The desc and body components are required for all Items; set any other components individually
/// after creating the item with new()
#[derive(Clone, Debug, Default)]
pub struct ItemData {
	desc:     Description, // Required for item generation
	body:     Body, // Required for item generation
	actions:  Option<ActionSet>, // Not strictly required but there's no other facility for context interaction
	// These are complex components that will require some kind of input for creation
	device:   Option<Device>,
	key:      Option<Key>,
	lock:     Option<Lockable>,
	opaque:   Option<Opaque>,
	open:     Option<Openable>,
	portable: Option<Portable>,
	viewshed: Option<Viewshed>,
	// These are just tags, all that is required is to create the component
	access:   Option<AccessPort>,
	backdrop: Option<Facade>,
	contain:  Option<Container>,
	carried:  Option<IsCarried>,
	memory:   Option<Memory>,
	mobile:   Option<Mobile>,
	network:  Option<Networkable>,
	obstruct: Option<Obstructive>,
	planq:    Option<Planq>,
}
impl ItemData {
	pub fn new(new_desc: Description, new_body: Body) -> ItemData {
		ItemData {
			desc: new_desc,
			body: new_body,
			..ItemData::default()
		}
	}
}
//   ##: THE ITEM DICTIONARY
//    #: ItemDict
/// Container struct for managing the dictionaries of furniture and furniture sets
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ItemDict {
	pub furniture: Vec<RawItem>,
	pub sets: Vec<RawItemSet>,
}
//    #: RawItem
/// Contains the item's definition as it was imported from external storage, to be converted to an internal type
/// It's generally less work to store the data as a big pile of strings and then do the conversion later
/// Even more later on I may decide to collapse this into one step but for now this is easier
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RawItem {
	pub name: String,
	pub desc: String,
	pub body: Vec<String>,
	pub shapes: Vec<Vec<String>>,
	pub extra: Vec<String>,
	pub constraints: Option<Vec<(String, String)>>
}
//    #: RawItemSet
/// Contains a definition for a set of items, such as a set of lockers, to facilitate spawning
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RawItemSet {
	pub name: String,
	pub contents: Vec<(String, String)>, // list of ('id', 'item_name'), indicates what to put where
	pub shapes: Vec<Vec<String>>, // Works same as the RawItem.shapes
}

//  ###: SIMPLE TYPES AND HELPERS
/// Loads the various furniture generation definitions from the external storage
pub fn load_furniture_defns(items_filename: &str, sets_filename: &str) -> ItemDict {
	// Make an empty ItemDict
	let mut new_dict = ItemDict::default();
	// Get a handle on the file to be loaded
	let item_file = File::open(items_filename).unwrap();
	// Open a reader object for the file handle
	let item_reader = BufReader::new(item_file);
	// If reading any of the lines failed, return a default dict
	new_dict.furniture = match serde_json::from_reader(item_reader) {
		//Ok(output) => {debug!("* recvd output: {:?}", output); output}, // DEBUG: log the successful output
		Ok(output) => {output},
		Err(e) => {error!("! could not create ItemDict.furniture: {}", e); Vec::new()},
	};
	// Construct the furniture set dictionary in the same way
	let sets_file = File::open(sets_filename).unwrap();
	let sets_reader = BufReader::new(sets_file);
	new_dict.sets = match serde_json::from_reader(sets_reader) {
		//Ok(output) => {debug!("* new sets: {:?}", output); output}, // DEBUG: log the successful output
		Ok(output) => {output},
		Err(e) => {error!("! could not create ItemDict.sets: {}", e); Vec::new()}
	};
	// Now return the dict from this function (or put it where it needs to go)
	new_dict
}

//  ###: OLD METHOD: predefined item types and bundles
/// Defines the set of item types, which allow requests to be made for specific types of items at runtime
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ItemType {
	#[default]
	Simple,  /// aka Item, name changed for better disambiguation
	Thing,
	Snack,
	Fixture,
	Furniture,
	Scenery,
	Door,
	Planq,
}
/// Defines a baseline 'inanimate object' component bundle
/// This is only useful on its own for defining pieces of scenery/backdrop, ie
/// things that will not move, do not have interactions, and do not block movement or sight
#[derive(Bundle)]
pub struct Item {
	pub desc:    Description,
	pub actions: ActionSet,
}
/// Defines the class of objects that are generally smaller than the player/assumed to be Portable
#[derive(Bundle)]
pub struct Thing {
	pub item:       Item,
	pub portable:   Portable,
}
/// just a demo thing for now, might change later
#[derive(Bundle)]
pub struct Snack {
	pub item:       Thing,
//	pub consume:    Consumable,
}
/// Defines the class of objects that are generally larger than the player/assumed to Obstruct movement
#[derive(Bundle)]
pub struct Fixture {
	pub item:       Item,
	pub obstructs:  Obstructive,
	pub opaque:     Opaque,
}
/// Defines the class of objects that allow/obstruct entity movement across a threshold
#[derive(Bundle)]
pub struct Door {
	pub item:       Fixture,
	pub door:       Openable,
	pub lock:       Lockable,
}


// EOF
