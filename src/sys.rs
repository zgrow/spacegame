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
pub fn movement_system(mut ereader:     EventReader<GameEvent>,
	                     model:           Res<Model>,
	                     mut msglog:      ResMut<MessageLog>,
	                     mut p_posn_res:  ResMut<Position>,
	                     mut p_query:     Query<(&mut Position, &mut Viewshed), With<Player>>,
	                     enty_query:      Query<(&Position, &Name, Option<&mut Viewshed>), Without<Player>>,
	                     //Query<(&Position, &Name, Option<&mut Viewshed>), (With<Obstructive>, Without<Player>)>,
) { // Note that these Events are custom jobbers, see the GameEvent enum in the components
	for event in ereader.iter() {
		//eprintln!("player attempting to move"); // DEBUG:
		match event.etype {
			PlayerMove(dir) => {
				let mut feedback;
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
				let t_index = model.levels[target.z as usize].to_index(target.x, target.y);
				// NOTE: IF the actor is changing z-levels, some extra logic is required:
				if dir == Direction::UP || dir == Direction::DOWN {
					// Prevent movement if an invalid z-level was obtained
					if target.z < 0 || target.z as usize >= model.levels.len() { continue; }
					// Prevent movement if the entity is not on a stairway
					let p_index = model.levels[p_pos.z as usize].to_index(p_pos.x, p_pos.y);
					if model.levels[p_pos.z as usize].tiles[p_index].ttype != TileType::Stairway {
						feedback = "There is nothing here to ".to_string();
						if zdiff == 1 { feedback.push_str("ascend.") }
						else { feedback.push_str("descend.") }
						msglog.tell_player(feedback);
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
				assert!(model.levels[target.z as usize].tiles.len() > 1, "Destination map is empty!");
				if model.levels[target.z as usize].blocked_tiles[t_index] {
					// Find out who's in the way and tell the player about it
					// CASE 1: there's an entity at that location
					for guy in enty_query.iter() {
						if guy.0 == &target {
							msglog.tell_player(format!("The way {} is blocked by a {}.", dir, guy.1));
							return;
						}
					}
					// CASE 2: it's a wall or similar
					msglog.tell_player(format!("The way {} is blocked by the {}.",
						              dir, &model.levels[target.z as usize].tiles[t_index].ttype.to_string()));
					return;
				}
				// If we arrived here, there's nothing in that space blocking the movement
				// Therefore, update the player's position
				(p_pos.x, p_pos.y, p_pos.z) = (target.x, target.y, target.z);
				// Don't forget to update the player position Resource too
				(p_posn_res.x, p_posn_res.y, p_posn_res.z) = (target.x, target.y, target.z);
				// Make sure the player's viewshed will be updated on the next pass
				p_view.dirty = true;
				// TODO: Tell the player about anything they can now see, such as the contents of the floor
				// A tile's contents are implicitly defined as those non-blocking entities at a given Posn
				let mut contents = Vec::new();
				for enty in enty_query.iter() {
					if *enty.0 == *p_pos {
						contents.push(&enty.1.name);
					}
				}
				if contents.len() >= 1 {
					if contents.len() <= 3 {
						// Use a short summary
						let mut text = "There's a ".to_string();
						loop {
							text.push_str(contents.pop().unwrap());
							if contents.len() == 0 { break; }
							else { text.push_str(", and a "); }
						}
					text.push_str(" here.");
					msglog.tell_player(text);
					} else {
						// Use a long summary
						msglog.tell_player(format!("There's some stuff here on the ground."));
					}
				}
				//let qty_entys_in_target = model.levels[target.z as usize].contents.len();
				//if qty_entys_in_target > 0 {
				//	if qty_entys_in_target >= 4 {
				//		msglog.tell_player(format!("There are several things lying here on the floor."));
				//	} else {
				//	// FIXME: display shortlist summary of items instead of the below
				//	}
				//}
			}
			// TODO: this is where we'd handle an NPCMove action
			_ => { } // Throw out anything we're not specifically interested in
		}
	}
}
/// Provides a map of blocked tiles, among other things, to the pathfinding systems
pub fn map_indexing_system(_ereader:    EventReader<GameEvent>,
	                         mut model:   ResMut<Model>,
	                         mut blocker_query: Query<&Position, With<Obstructive>>,
	                         _enty_query:  Query<(Entity, &Position)>
) {
	// ERROR: This system is currently hardcoded for level 0!
	// TODO: consider possible optimization for not updating levels that the player is not on?
	// might require some extra smartness to allow updates if the LMR does something out of sight
	// First, rebuild the blocking map by the map tiles
	model.levels[0].update_blocked_tiles();
	// Then, step through all blocking entities and flag their locations on the map as well
	for guy in blocker_query.iter_mut() {
		if guy.z != 0 { continue; } // FIXME: this only allows updates to the blocking map for floor 0
		let index = model.levels[0].to_index(guy.x, guy.y);
		model.levels[0].blocked_tiles[index] = true;
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
			let map = &mut model.levels[posn.z as usize];
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
			ItemPickup(Creature::Player) => {
				msglog.add("Player attempted to GET item".to_string(), "world".to_string(), 1, 1);
			}
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
