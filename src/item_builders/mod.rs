// items.rs
// Describes the class of inanimate objects throughout the game, both Props and Furniture

use bevy::prelude::*;
use bevy::ecs::world::EntityMut;
use crate::components::*;
use crate::components::Name;

/// Defines the set of item types, which allow requests to be made for specific types of items at runtime
pub enum ItemType {
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
	pub name:   Name,
	pub posn:   Position,
	pub render: Renderable,
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
	pub can_block:  Obstructive,
}
/// Defines the class of objects that allow/obstruct entity movement across a threshold
#[derive(Bundle)]
pub struct Door {
	pub item:       Fixture,
	pub door:       Openable
}

/// Provides a facility for creating items during gameplay
#[derive(Resource, Default)]
pub struct ItemBuilder { pub spawn_count: i32 }
impl<'a, 'b> ItemBuilder where 'a: 'b {
	/// Spawns an Item Entity in the World, ie at a map Position, and returns a ref to it
	pub fn spawn(&'b mut self, world: &'a mut World, new_type: ItemType, location: Position) -> EntityMut<'b> {
		self.spawn_count += 1;
		match new_type {
			ItemType::Simple    => {
				world.spawn( Item {
					name: Name { name: format!("_simpleItem_{}", self.spawn_count) },
					posn:   location,
					render: Renderable { glyph: "i".to_string(), fg: 4, bg: 0 },
				})
			}
			ItemType::Thing     => {
				world.spawn( Thing {
					item: Item {
						name: Name { name: format!("_thing_{}", self.spawn_count) },
						posn:   location,
						render: Renderable { glyph: "t".to_string(), fg: 4, bg: 0 },
					},
					portable: Portable { carrier: Entity::PLACEHOLDER },
				})
			}
			ItemType::Fixture   => {
				world.spawn( Fixture {
					item: Item {
						name: Name { name: format!("_fixture_{}", self.spawn_count) },
						posn:   location,
						render: Renderable { glyph: "#".to_string(), fg: 4, bg: 0 },
					},
					can_block: Obstructive { },
				})
			}
			ItemType::Door      => {
				world.spawn( Door {
					item: Fixture {
						item:   Item {
							name: Name { name: format!("_door_{}", self.spawn_count) },
							posn:   location,
							render: Renderable { glyph: "O".to_string(), fg: 4, bg: 0 },
						},
						can_block: Obstructive { },
					},
					door: Openable { is_open: true }
				})
			}
			ItemType::Snack     => {
				world.spawn( Snack {
					item: Thing {
						item: Item {
							name:   Name { name: format!("_snack_{}", self.spawn_count) },
							posn:   location,
							render: Renderable { glyph: "%".to_string(), fg: 5, bg: 0 },
						},
						portable: Portable { carrier: Entity::PLACEHOLDER },
					},
					//consume: Consumable { },
				})
			}
		}
	}
	// Spawns an Item in a specified Container, such as player's inventory or inside a box
	//pub fn give(&self, new_item: String, target: ResMut<Entity>) { } // TODO:
}


// EOF
