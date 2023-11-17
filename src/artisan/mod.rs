// artisan/mod.rs
// Describes the class of inanimate objects throughout the game, both Props and Furniture

// CLIPPY SHUT UPPPPPPPPPP
#![allow(unused_variables)]
#![allow(dead_code)]

// *** EXTERNAL LIBRARIES
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
use bevy::utils::hashbrown::HashMap;

// *** INTERNAL LIBRARIES
//use crate::camera::*;
use crate::components::*;
use crate::engine::planq::*;
use crate::mason::logical_map::*;
use crate::worldmap::*;
use furniture::Facade;

pub mod furniture;

/// Provides a facility for creating items during gameplay
#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct ItemBuilder {
	pub spawn_count: i32,
	body:     Option<Body>,
	desc:     Option<Description>,
	actions:  Option<ActionSet>,
	// Optional/auxiliary components
	access:   Option<AccessPort>,
	contain:  Option<Container>,
	device:   Option<Device>,
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
	dict:     ItemDict,
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
			dict: load_furniture_defns("resources/furniture_v2.json"),
			..ItemBuilder::default()
		}
	}
	pub fn create(&mut self, new_item: &str) -> &mut ItemBuilder {
		debug!("* ItemBuilder request: '{}'", new_item);
		if let Some(item_data) = self.dict.furniture.iter().find(|x| x.name == new_item) {
			self.desc = Some(Description::new().name(&item_data.name).desc(&item_data.desc));
			for line in item_data.body.iter() {
				debug!("*** {}", line);
			}
			debug!("* recvd item_data.body: {:?}", item_data.body.clone());
			self.body = Some(Body::new_from_str(item_data.body.clone()));
			// FIXME: need to import the placement pattern *here* as self.shape
			if !item_data.extra.is_empty() {
				// Parse and add any additional components that are in the item's definition
				//debug!("* recvd item_data.extra: {:?}", item_data.extra);
				for component in item_data.extra.iter() {
					//debug!("* raw component value: {}", component);
					// HINT: This will in fact return the entire string if the string consists of only a single word
					//let new_string: Vec<&str> = component.split(' ').collect();
					let mut new_cmpnt = component.split(' ');
					let part = new_cmpnt.next().unwrap();
					let details: Vec<&str> = new_cmpnt.collect();
					let error_msg = "! ERROR: Could not parse key:value for ";
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
						_ => { error!("! ERROR: requested component {} was not recognized", component); }
					}
				}
			}
		} else {
			error!("! ERROR: item request '{}' not found in dictionary!", new_item);
		}
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
	pub fn at(&mut self, posn: Position) -> &mut ItemBuilder {
		if let Some(body) = self.body.as_mut() {
			body.move_to(posn);
		}
		self
	}
	pub fn give_to(&mut self, target: Entity) -> &mut ItemBuilder {
		self.portable = Some(Portable::new(target));
		self
	}
	/// Constructs the item into the specified Bevy::App, and returns the generated Entity ID as well as the full set
	/// of Positions, aka the Body.extent, aka the item's shape, that the item occupies on the map
	pub fn build(&'b mut self, world: &'a mut World) -> (EntityMut<'b>, Vec<Position>) {
		self.spawn_count += 1;
		let mut item_shape = Vec::new();
		let mut new_item = world.spawn_empty();
		// Add all of the populated components to the new entity
		if let Some(desc)     = &self.desc { new_item.insert(desc.clone()); self.desc = None; }
		if let Some(body)     = &self.body {
			//debug!("* creating new item {} with shape {:?}", body.posns());
			item_shape = body.posns();
			new_item.insert(body.clone()); self.body = None;
		}
		if let Some(actions)  = &self.actions { new_item.insert(actions.clone()); self.actions = None; }
		if let Some(obstruct) = self.obstruct { new_item.insert(obstruct); self.obstruct = None; }
		if let Some(opaque)   = self.opaque { new_item.insert(opaque); self.opaque = None; }
		if let Some(open)     = &self.open { new_item.insert(open.clone()); self.open = None; }
		if let Some(portable) = self.portable { new_item.insert(portable); self.portable = None; }
		if let Some(device)   = self.device { new_item.insert(device); self.device = None; }
		if let Some(mobile)   = self.mobile { new_item.insert(mobile); self.mobile = None; }
		if let Some(contain)  = &self.contain { new_item.insert(*contain); self.contain = None; }
		if let Some(lock)     = self.lock { new_item.insert(lock); self.lock = None; }
		if let Some(key)      = self.key { new_item.insert(key); self.key = None; }
		if let Some(planq)    = self.planq { new_item.insert(planq); self.planq = None; }
		if let Some(backdrop) = self.backdrop { new_item.insert(backdrop); self.backdrop = None; }
		(new_item, item_shape)
	}
	/// Generates the list of decorative items that the worldgen will need to spawn
	pub fn decorate(&mut self, worldmap: &Model) -> Vec<Position> {
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
		todo!("Not done implementing this yet");
	}
}

/// Loads the various furniture generation definitions from the external storage
pub fn load_furniture_defns(filename: &str) -> ItemDict {
	// Get a handle on the file to be loaded
	let file = File::open(filename).unwrap();
	// Open a reader object for the file handle
	let reader = BufReader::new(file);
	let value: ItemDict = match serde_json::from_reader(reader) {
		//Ok(output) => {debug!("* recvd: {:?}", output); output},
		Ok(output) => {output},
		Err(e) => {debug!("! ERROR: load_furniture_defns() failed: {}", e); ItemDict::default()},
	};
	// Now return the dict from this function (or put it where it needs to go)
	value
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ItemDict {
	pub furniture: Vec<RawItem>
}

/// Contains the item's definition as it was imported from external storage, to be converted to an internal type
/// It's generally less work to store the data as a big pile of strings and then do the conversion later
/// Even more later on I may decide to collapse this into one step but for now this is easier
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RawItem {
	pub name: String,
	pub desc: String,
	pub body: Vec<String>,
	pub shape: Vec<String>,
	pub extra: Vec<String>,
}

/* A struct defn like so:
 *   pub struct RawItem {
 *     pub description: Vec<String>,
 *     pub body: Vec<String>,
 *     pub placement: String,
 *     pub components: Option<ComponentBundle>,
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
 * Note that in the example above, the ComponentBundle type would need to be defined
 * For that matter, *any* JSON dictionary becomes a struct with named fields in Rust
 * If the field names cannot be exact matches, use serde macros to handle renaming
 */
// Describes how to construct and place an object in the game world, such as for mapgen
#[derive(Clone, Debug, Default)]
pub struct ItemPattern {
	pub name: String,
	pub shape: HashMap<Position, GraphCell>,
}
impl ItemPattern {
	pub fn new() -> ItemPattern {
		ItemPattern::default()
	}
}
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

/// Passing this data structure to an ItemBuilder will take care of the entire item creation request
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

// OLD METHOD: predefined item types and bundles
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
