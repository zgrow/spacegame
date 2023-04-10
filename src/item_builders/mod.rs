// items.rs
// Describes the class of inanimate objects throughout the game, both Props and Furniture

use bevy::prelude::*;
use bevy::ecs::world::EntityMut;
use crate::components::*;
use crate::components::Name;

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
/// Defines the class of objects that are generally larger than the player/assumed to Obstruct movement
#[derive(Bundle)]
pub struct Fixture {
	pub item:       Item,
	pub can_block:  Obstructive,
}
#[derive(Bundle)]
pub struct Door {
	pub item:       Fixture,
	pub door:       Openable
}
/// Defines the set of item types, which allow requests to be made for specific types of items at runtime
pub enum ItemType {
	Simple,
	Thing,
	Fixture,
	Door,
}

/// Provides a facility for creating items during gameplay
#[derive(Resource, Default)]
pub struct ItemBuilder { }
impl<'a, 'b> ItemBuilder where 'a: 'b {
	/// Spawns an Item Entity in the World, ie at a map Position, and returns a ref to it
	pub fn spawn(&'b self, world: &'a mut World, new_type: ItemType, location: Position) -> EntityMut<'b> {
		match new_type {
			ItemType::Simple  => {
				world.spawn( Item {
					name: Name { name: "simpleItem".to_string() },
					posn:   location,
					render: Renderable { glyph: "i".to_string(), fg: 4, bg: 0 },
				})
			}
			ItemType::Thing   => {
				world.spawn( Thing {
					item: Item {
						name: Name { name: "thing".to_string() },
						posn:   location,
						render: Renderable { glyph: "t".to_string(), fg: 4, bg: 0 },
					},
					portable: Portable { },
				})
			}
			ItemType::Fixture => {
				world.spawn( Fixture {
					item: Item {
						name: Name { name: "fixture".to_string() },
						posn:   location,
						render: Renderable { glyph: "#".to_string(), fg: 4, bg: 0 },
					},
					can_block: Obstructive { },
				})
			}
			ItemType::Door    => {
				world.spawn( Door {
					item: Fixture {
						item:   Item {
							name: Name { name: "door".to_string() },
							posn:   location,
							render: Renderable { glyph: "O".to_string(), fg: 4, bg: 0 },
						},
						can_block: Obstructive { },
					},
					door: Openable { is_open: true }
				})
			}
		}
	}
	// Spawns an Item in a specified Container, such as player's inventory or inside a box
//	pub fn give(&self, new_item: String, target: ResMut<Entity>) { } // TODO:

}

// EOF
