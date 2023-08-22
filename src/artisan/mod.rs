// artisan/mod.rs
// Describes the class of inanimate objects throughout the game, both Props and Furniture

// *** EXTERNAL LIBRARIES
use bevy::prelude::*;
use bevy::ecs::world::EntityMut;

// *** INTERNAL LIBRARIES
use crate::components::*;
//use crate::components::ActorName;

/// Defines the set of item types, which allow requests to be made for specific types of items at runtime
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ItemType {
	#[default]
	Simple,  /// aka Item, name changed for better disambiguation
	Thing,
	Snack,
	Fixture,
	Door,
}
/// Defines a baseline 'inanimate object' component bundle
/// This is only useful on its own for defining pieces of scenery/backdrop, ie
/// things that will not move, do not have interactions, and do not block movement or sight
#[derive(Bundle)]
pub struct Item {
	pub name:    ActorName,
	pub desc:    Description,
	pub render:  Renderable,
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
}

/// Provides a facility for creating items during gameplay
#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct ItemBuilder { pub spawn_count: i32 }
impl<'a, 'b> ItemBuilder where 'a: 'b {
	/// Spawns an Item Entity in the World, ie at a map Position, and returns a ref to it
	pub fn spawn_at(&'b mut self, world: &'a mut World, new_type: ItemType, location: Position) -> EntityMut<'b> {
		self.spawn_count += 1;
		//eprintln!("* spawning object {new_type:?} at {}", location); // DEBUG: announce new object spawn
		match new_type {
			ItemType::Simple    => {
				world.spawn((
					Item {
						name: ActorName { name: format!("_simpleItem_{}", self.spawn_count) },
						desc: Description::new(format!("_simpleItem_{}", self.spawn_count), "A simple Item.".to_string()),
						render: Renderable { glyph: "i".to_string(), fg: 4, bg: 0 },
						actions: ActionSet::new(),
					},
					Position::from(location)
				))
			}
			ItemType::Thing     => {
				world.spawn((
					Thing {
						item: Item {
							name: ActorName { name: format!("_thing_{}", self.spawn_count) },
							desc: Description::new(format!("_thing_{}", self.spawn_count), "A new Thing.".to_string()),
							render: Renderable { glyph: "t".to_string(), fg: 4, bg: 0 },
							actions: ActionSet::new(),
						},
						portable: Portable { carrier: Entity::PLACEHOLDER },
					},
					Position::from(location)
				))
			}
			ItemType::Fixture   => {
				world.spawn((
					Fixture {
						item: Item {
							name: ActorName { name: format!("_fixture_{}", self.spawn_count) },
							desc: Description::new(format!("_fixture_{}", self.spawn_count), "A plain Fixture.".to_string()),
							render: Renderable { glyph: "#".to_string(), fg: 4, bg: 0 },
							actions: ActionSet::new(),
						},
						obstructs: Obstructive { },
						opaque:    Opaque { opaque: true },
					},
					Position::from(location)
				))
			}
			ItemType::Door      => {
				world.spawn((
					Door {
						item: Fixture {
							item:   Item {
								name: ActorName { name: format!("_door_{}", self.spawn_count) },
								desc: Description::new(format!("_door_{}", self.spawn_count), "A regular Door.".to_string()),
								render: Renderable { glyph: "█".to_string(), fg: 4, bg: 0 },
								actions: ActionSet::new(),
							},
							obstructs: Obstructive { },
							opaque:    Opaque { opaque: true },
						},
						door: Openable {
							is_open: false,
							open_glyph: "▔".to_string(),
							closed_glyph: "█".to_string(),
						}
					},
					Position::from(location)
				))
			}
			ItemType::Snack     => {
				world.spawn((
					Snack {
						item: Thing {
							item: Item {
								name:   ActorName { name: format!("_snack_{}", self.spawn_count) },
								desc:   Description::new(format!("_snack_{}", self.spawn_count), "A tasty Snack.".to_string()),
								render: Renderable { glyph: "%".to_string(), fg: 5, bg: 0 },
								actions: ActionSet::new(),
							},
							portable: Portable { carrier: Entity::PLACEHOLDER },
						},
						//consume: Consumable { },
					},
					Position::from(location)
				))
			}
		}
	}
	/// Spawns an Item in a specified Container, such as player's inventory or inside a box
	pub fn spawn_to(&'b mut self, world: &mut World, new_item: ItemType, target: Entity) {
		eprintln!("* giving new item {:?} to target entity {:?}", new_item, target); // DEBUG: announce new item creation
		self.spawn_count += 1;
		match new_item {
			ItemType::Snack     => {
				world.spawn(
					Snack {
						item: Thing {
							item: Item {
								name:   ActorName { name: format!("_snack_{}", self.spawn_count) },
								desc:   Description::new(format!("_snack_{}", self.spawn_count), "A tasty Snack.".to_string()),
								render: Renderable { glyph: "%".to_string(), fg: 5, bg: 0 },
								actions: ActionSet::new(),
							},
							portable: Portable { carrier: target },
						},
						//consume: Consumable { },
					}
				);
			}
			_ => {
				eprintln!("* ERR: cannot give non-Snack items to entities"); // DEBUG: report failure to spawn item
			}
		}
	}
	/// Calls spawn() repeatedly for each item on the given list
	pub fn spawn_batch(&'b mut self, world: &'a mut World, items: &mut Vec<(ItemType, Position)>, z_level: i32) {
		//eprintln!("* spawning batch: {} items", items.len()); // DEBUG: announce batch item spawn
		for item in items {
			item.1.z = z_level;
			self.spawn_at(world, item.0, item.1);
		}
	}
}

// EOF
