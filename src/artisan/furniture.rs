// artisan/furniture.rs
// Details on Furniture and Scenery: objects that do not ever move, and are often merely for flavor or setting

// The Furniture and Scenery objects fit into the Item component hierarchy like so:
//  Item
//    Thing
//      Fixture
//        Furniture
//        Scenery
// QUERY: It might be worth adapting Doors to be a special case of Furniture

// The simplest possible object worth representing in the game world should have the following components:
// - Description: provides the item's name, required for environment
// - Body: provides the item's visual representation and canonical position, required for environment
// - ActionSet: provides interaction context: not required for anything that the player shouldn't interact with
// Any other fields are optional for functionality but if needed, must be provided by the item defn

use bevy::prelude::*;

use crate::artisan::*;

/// Tag Component for marking Scenery objects; used mostly to exclude them from query results
#[derive(Component, Copy, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Facade { }

#[derive(Bundle)]
pub struct Furniture {
	pub item:     Fixture, // incl: Item(Description, Renderable, ActionSet), Obstructive, Opaque
}

#[derive(Bundle)]
pub struct Scenery {
	pub backdrop:  Facade, // tag Component to make exclusion from queries easier
	//pub render:    Renderable,
	pub obstruct:  Obstructive,
	pub opaque:    Opaque,
}

// EOF
