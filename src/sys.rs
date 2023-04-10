// sys.rs
// Defines the various subsystems we'll be running on the GameEngine

// NOTE: see bevy/examples/games/alien_cake_addict.rs for example on handling the Player entity

use crate::components::*;
use crate::camera_system::CameraView;
use crate::map::*;
use crate::components::{Name, Position, Renderable, Player, Mobile};
use crate::sys::GameEventType::*;
use crate::app::messagelog::MessageLog;
use crate::item_builders::*;
use bevy::ecs::system::{Commands, Res, Query, ResMut};
use bevy::ecs::event::EventReader;
use bevy::ecs::query::{With, Without};
use bevy::ecs::entity::Entity;
use bracket_pathfinding::prelude::*;

//  UTILITIES
/// Converts a spacegame::Position into a bracket_pathfinding::Point
pub fn posn_to_point(input: &Position) -> Point { Point { x: input.x, y: input.y } }

//  SINGLETON SYSTEMS (run once)
/// Spawns a new CameraView on the game world (ie the default/main view)
pub fn new_camera_system(mut commands: Commands) {
	commands.insert_resource(CameraView {
		map: Vec::new(),
		width: 0,
		height: 0,
	});
}

/// Spawns a new player, including their subsystems and default values
pub fn new_player_spawn(mut commands: Commands,
	                     spawnpoint: Res<Position>,
	                     mut msglog: ResMut<MessageLog>,
) {
	commands.spawn((
		// this is the player's collection of components and their initial values
		Player      { },
		Name        {name: "Pleyeur".to_string()},
		Position    {x: spawnpoint.x, y: spawnpoint.y, z: spawnpoint.z},
		Renderable  {glyph: "@".to_string(), fg: 2, bg: 0},
		Viewshed    {visible_tiles: Vec::new(), range: 8, dirty: true},
		Mobile      { },
		Obstructive { },
	));
	msglog.add("WELCOME TO SPACEGAME".to_string(), "world".to_string(), 1, 1);
}
/// Spawns a new LMR at the specified Position, using default values
pub fn new_lmr_spawn(mut commands: Commands)
{
	commands.spawn((
		Name        {name: "LMR".to_string()},
		Position    {x: 12, y: 12, z: 0}, // TODO: remove magic numbers
		Renderable  {glyph: "l".to_string(), fg: 14, bg: 0},
		Viewshed    {visible_tiles: Vec::new(), range: 5, dirty: true},
		Mobile      { },
		Obstructive { },
	));
}
/// Spawns the player's PLANQ [TODO: in the starting locker]
pub fn new_planq_spawn(mut commands: Commands)
{
	commands.spawn((
		Planq { },
		Thing {
			item: Item {
				name: Name { name: "PLANQ".to_string() },
				posn: Position::new(25, 30, 0),
				render: Renderable { glyph: "¶".to_string(), fg: 3, bg: 0 },
			},
			portable: Portable { },
		}
	));
}

//  CONTINUOUS SYSTEMS (run frequently)
/// Handles entities that can move around the map
pub fn movement_system(mut ereader: EventReader<GameEvent>,
                       model: Res<Model>,
                       mut p_query: Query<(&mut Position, &mut Viewshed), With<Player>>,
                       mut p_posn_res: ResMut<Position>,
                       npc_query: Query<((&Position, &mut Mobile, Option<&mut Viewshed>), (Without<Player>, With<Mobile>))>,
                       blocker_query: Query<&Position, (With<Obstructive>, Without<Player>, Without<Mobile>)>,
) {
	// Note that these Events are custom jobbers, see the GameEvent enum in the components
	for event in ereader.iter() {
		//eprintln!("player attempting to move"); // DEBUG:
		match event.etype {
			PlayerMove(dir) => {
				let (mut p_pos, mut p_view) = p_query.single_mut();
				let mut xdiff = 0;
				let mut ydiff = 0;
				let mut zdiff = 0; // NOTE: not a typical component: z-level indexes to map stack
				match dir {
					Direction::N    =>             { ydiff -= 1 }
					Direction::NW   => { xdiff -= 1; ydiff -= 1 }
					Direction::W    => { xdiff -= 1 }
					Direction::SW   => { xdiff -= 1; ydiff += 1 }
					Direction::S    =>             { ydiff += 1 }
					Direction::SE   => { xdiff += 1; ydiff += 1 }
					Direction::E    => { xdiff += 1 }
					Direction::NE   => { xdiff += 1; ydiff -= 1 }
					Direction::UP   =>      { zdiff += 1 }
					Direction::DOWN =>      { zdiff -= 1 }
				}
				// Calculate the desired position's components
				let mut target = Position{x: p_pos.x + xdiff, y: p_pos.y + ydiff, z: p_pos.z + zdiff};
				// If trying to move up/down to a different floor: (ie zdiff != 0)
				if dir == Direction::UP || dir == Direction::DOWN {
					// Prevent movement if an invalid z-level was obtained
					if target.z < 0 || target.z as usize >= model.levels.len() { continue; }
					// Prevent movement if the entity is not on a stairway
					let t_index = model.levels[p_pos.z as usize].to_index(p_pos.x, p_pos.y);
					if model.levels[p_pos.z as usize].tiles[t_index].ttype != TileType::Stairway {
						continue;
					}
					// If we arrived here, then all's good; get the destination coords
					let possible = model.portals.get(&(p_pos.x, p_pos.y, p_pos.z));
					//eprintln!("poss: {possible:?}"); // DEBUG:
					if possible.is_some() {
						let actual = possible.unwrap();
						target.x = actual.0;
						target.y = actual.1;
						target.z = actual.2;
					}
				}
				assert!(model.levels[target.z as usize].tiles.len() > 1, "destination map is empty!");
				// Check for NPC collisions
				for guy in npc_query.iter() { if *guy.0.0 == target { return; } }
				// Check for immobile entity collisions
				for blocker in blocker_query.iter() { if *blocker == target { return; } }
				// Check for map collisions
				if model.levels[target.z as usize].is_occupied(target) { return; }
				//eprintln!("target: {target:?}"); // DEBUG:
				// If we arrived here, there's nothing in that space blocking the movement
				// Therefore, update the player's position
				(p_pos.x, p_pos.y, p_pos.z) = (target.x, target.y, target.z);
				// Don't forget to update the player position Resource too
				(p_posn_res.x, p_posn_res.y, p_posn_res.z) = (target.x, target.y, target.z);
				// Make sure the player's viewshed will be updated on the next pass
				p_view.dirty = true;
			}
			// TODO: this is where we'd handle an NPCMove action
			_ => { } // Throw out anything we're not specifically interested in
		}
	}
}
/// Handles entities that can see physical light
pub fn visibility_system(mut model: ResMut<Model>,
                         mut seers: Query<(&mut Viewshed, &Position, Option<&Player>)>
) {
	for (mut viewshed, posn, player) in &mut seers {
		//eprintln!("posn: {posn:?}"); // DEBUG:
		if viewshed.dirty {
			assert!(posn.z != -1);
			let map = &mut model.levels[posn.z as usize]; // ERROR: here
			viewshed.visible_tiles.clear();
			viewshed.visible_tiles = field_of_view(posn_to_point(posn), viewshed.range, map);
			viewshed.visible_tiles.retain(|p| p.x >= 0 && p.x < map.width
				                           && p.y >= 0 && p.y < map.height
			);
			if let Some(_player) = player { // if this is the player...
				for posn in &viewshed.visible_tiles { // For all the player's visible tiles...
					// ... set the corresponding tile in the map.revealed_tiles to TRUE
					let map_index = map.to_index(posn.x, posn.y);
					map.revealed_tiles[map_index] = true;
				}
			}
			viewshed.dirty = false;
		}
	}
}
/// Handles pickup/drop/destroy requests for Items
pub fn item_collection_system(mut ereader: EventReader<GameEvent>,
	                          mut _model: ResMut<Model>,
	                          mut _entity_query: Query<(Entity, &Position, &Container)>,
	                          mut msglog: ResMut<MessageLog>,
) {
	for event in ereader.iter() {
		match event.etype {
			ItemPickup(Creature::Player) => { msglog.add("Player attempted to GET item".to_string(), "world".to_string(), 1, 1); }
			_ => { }
		}
	}
}

/// Allows us to run PLANQ updates and methods in their own thread, just like a real computer~
pub fn planq_system(_ereader: EventReader<GameEvent>, // subject to change
	                _p_query: Query<&Position, With<Player>>, // provides interface to player data
	                //planq: ResMut<Planq>? // contains the PLANQ's settings and data storage
) {
	/* TODO: Implement level generation such that the whole layout can be created at startup from a
	 * tree of rooms, rather than by directly loading a REXPaint map; by retaining this tree-list
	 * of rooms in the layout, the PLANQ can then show the player's location as an output
	 */

}

/* TODO: "memory_system":
 * Maintains an enhanced Map of Tiles where the Tile glyphs are painted to include the locations of
 * existing Renderables in addition to the terrain
 * When this system is initialized (after the initial level setup, before the disaster design
 * phase), it provides a 'prior memory' of the ship layout
 * When this system is updated, it provides the player with a visual mapping of where to find
 * complex machines and other gameplay objectives
 */

// EOF
