// sys.rs
// Defines the various subsystems we'll be running on the GameEngine

// NOTE: see bevy/examples/games/alien_cake_addict.rs for example on handling the Player entity

use crate::components::*;
use crate::camera_system::CameraView;
use crate::map::*;
use crate::components::{Name, Position, Renderable, Player, Mobile};
use crate::components::PlanqEventType::*;
use crate::sys::GameEventType::*; // Required to avoid having to specify the enum path every time
use crate::app::messagelog::MessageLog;
use crate::app::planq::*;
use crate::app::*;
use crate::item_builders::*;
use bevy::ecs::system::{Commands, Res, Query, ResMut};
use bevy::ecs::event::EventReader;
use bevy::ecs::query::{With, Without};
use bevy::ecs::entity::Entity;
use bracket_pathfinding::prelude::*;

// TODO: Need to implement change detection on the following:
// map_indexing_system
// visibility_system

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
	                    mut msglog: ResMut<MessageLog>,
	                    spawnpoint: Res<Position>,
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
		Container   { contents: Vec::new() },
		Opaque      { opaque: true },
		CanOpen     { },
	));
	msglog.add("WELCOME TO SPACEGAME".to_string(), "world".to_string(), 1, 1);
}
/// Spawns a new LMR at the specified Position, using default values
pub fn new_lmr_spawn(mut commands:  Commands,
	                 mut msglog:    ResMut<MessageLog>,
) {
	commands.spawn((
		Name        {name: "LMR".to_string()},
		Position    {x: 12, y: 12, z: 0}, // TODO: remove magic numbers
		Renderable  {glyph: "l".to_string(), fg: 14, bg: 0},
		Viewshed    {visible_tiles: Vec::new(), range: 5, dirty: true},
		Mobile      { },
		Obstructive { },
		Opaque      { opaque: true },
		CanOpen     { },
	));
	msglog.add(format!("LMR spawned at {}, {}, {}", 12, 12, 0), "debug".to_string(), 1, 1);
}
/// Spawns the player's PLANQ [TODO: in the starting locker]
pub fn new_planq_spawn(mut commands:    Commands,
	                   mut msglog:      ResMut<MessageLog>,
) {
	commands.spawn((
		Planq { },
		Thing {
			item: Item {
				name: Name { name: "PLANQ".to_string() },
				posn: Position::new(25, 30, 0),
				render: Renderable { glyph: "¶".to_string(), fg: 3, bg: 0 },
			},
			portable: Portable { carrier: Entity::PLACEHOLDER },
		}
	));
	msglog.add(format!("planq spawned at {}, {}, {}", 25, 30, 0), "debug".to_string(), 1, 1);
}

//  CONTINUOUS SYSTEMS (run frequently)
/// Runs assessment of the game state for things like victory/defeat conditions, &c
pub fn engine_system(mut state:         ResMut<GameSettings>,
	                 mut ereader:       EventReader<GameEvent>,
	                 p_query:           Query<(Entity, &Position), With<Player>>,
	                 p_items_query:     Query<(Entity, &Portable), Without<Position>>,
	                 q_query:           Query<(Entity, &Portable), With<Planq>>,
) {
	for event in ereader.iter() {
		match event.etype {
			ModeSwitch(mode) => {// Immediately switch to the specified mode
				eprintln!("Switching engine mode: {mode:?}"); // DEBUG:
				state.mode = mode;
			}
			PauseToggle => {
				eprintln!("Pause toggled"); // DEBUG:
				if state.mode == EngineMode::Running { state.mode = EngineMode::Paused; }
				else if state.mode == EngineMode::Paused { state.mode = EngineMode::Running; }
			}
			_ => { } // Throw out all other event types
		}
	}
	// Check for the victory state
	let player = p_query.get_single().unwrap();
	let planq = q_query.get_single().unwrap();
	let mut p_inventory = Vec::new();
	for item in p_items_query.iter() {
		if item.1.carrier == player.0 { p_inventory.push(item.0); }
	}
	// version 0.1: Player must be standing in the specified Position
	//if *player.1 == Position::new(28, 1, 1) { state.mode = EngineMode::GoodEnd; }
	// version 0.2: v0.1 AND Player.has == planq
	if *player.1 == Position::new(28, 1, 1) && p_inventory.contains(&planq.0) {
		// FIXME: this state change is not propagating up to the actual engine mode variable
		eprintln!("VICTORY condition achieved!"); // DEBUG:
		state.mode = EngineMode::GoodEnd;
	}
}
/// Handles entities that can move around the map
pub fn movement_system(mut ereader:     EventReader<GameEvent>,
	                   mut msglog:      ResMut<MessageLog>,
	                   mut p_posn_res:  ResMut<Position>,
	                   mut p_query:     Query<(&mut Position, &mut Viewshed), With<Player>>,
	                   model:           Res<Model>,
	                   enty_query:      Query<(&Position, &Name, Option<&mut Viewshed>), Without<Player>>,
) {
	// NOTE: the enty_query doesn't need to include Obstructive component because the map's
	// blocked_tiles sub-map already includes that information in an indexed vector
	// This allows us to only worry about consulting the query when we know we need it, as it is
	// much more expensive to iterate a query than to generate it
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
				// A tile's contents are implicitly defined as those non-blocking entities at a given Posn
				// If we use the player's position, then we may conclude that any entities at that
				// position that are not the player must be non-blocking, since the player's
				// movement rules prevent them from entering a tile with any other Obstructive enty
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
			}
			// TODO: this is where we'd handle an NPCMove action
			_ => { } // Throw out anything we're not specifically interested in
		}
	}
}
/// Handles updates to the 'meta' maps, ie the blocked and opaque tilemaps
pub fn map_indexing_system(mut model:   ResMut<Model>,
	                       mut blocker_query: Query<&Position, With<Obstructive>>,
	                       mut opaque_query: Query<(&Position, &Opaque)>,
) {
	// TODO: consider possible optimization for not updating levels that the player is not on?
	// might require some extra smartness to allow updates if the LMR does something out of sight
	// First, rebuild the blocking map by the map tiles
	let mut f_index = 0;
	let mut index;
	for floor in model.levels.iter_mut() {
		floor.update_tilemaps(); // Update tilemaps based on their tiletypes
		// Then, step through all blocking entities and flag their locations on the map as well
		for guy in blocker_query.iter_mut() {
			if guy.z != f_index { continue; }
			index = floor.to_index(guy.x, guy.y);
			floor.blocked_tiles[index] = true;
		}
		// Do the same for the opaque entities
		for guy in opaque_query.iter_mut() {
			if guy.0.z != f_index { continue; }
			index = floor.to_index(guy.0.x, guy.0.y);
			floor.opaque_tiles[index] = guy.1.opaque;
		}
		f_index += 1;
	}
}
/// Handles ActorOpen/Close events
pub fn door_system(mut commands:    Commands,
	               mut ereader:     EventReader<GameEvent>,
	               mut msglog:      ResMut<MessageLog>,
	               mut door_query:  Query<(Entity, &Position, &mut Openable, &mut Renderable, &mut Opaque, Option<&Obstructive>)>,
	               mut e_query:     Query<(Entity, &Position, &Name, Option<&Player>, Option<&mut Viewshed>), With<CanOpen>>,
) {
	for event in ereader.iter() {
		if event.etype != ActorOpen
		&& event.etype != ActorClose { continue; }
		if event.context.is_none() { continue; }
		let econtext = event.context.as_ref().unwrap();
		eprintln!("actor opening door {0:?}", econtext.object); // DEBUG:
		let actor = e_query.get_mut(econtext.subject).unwrap();
		let player_action = actor.3.is_some();
		let mut message: String = "".to_string();
		match event.etype {
			GameEventType::ActorOpen => {
				//eprintln!("Trying to open a door"); // DEBUG:
				for mut door in door_query.iter_mut() {
					if door.0 == econtext.object {
						door.2.is_open = true;
						door.3.glyph = door.2.open_glyph.clone();
						door.4.opaque = false;
						commands.entity(door.0).remove::<Obstructive>();
					}
				}
				if player_action {
					message = "The door slides open at your touch.".to_string();
				} else {
					message = format!("The {} opens a door.", actor.2.name.clone());
				}
				if actor.4.is_some() { actor.4.unwrap().dirty = true; }
			}
			GameEventType::ActorClose => {
				//eprintln!("Trying to close a door"); // DEBUG:
				for mut door in door_query.iter_mut() {
					if door.0 == econtext.object {
						door.2.is_open = false;
						door.3.glyph = door.2.closed_glyph.clone();
						door.4.opaque = true;
						commands.entity(door.0).insert(Obstructive {});
					}
				}
				if player_action {
					message = "The door slides shut.".to_string();
				} else {
					message = format!("The {} closes a door.", actor.2.name.clone());
				}
				if actor.4.is_some() { actor.4.unwrap().dirty = true; }
			}
			_ => { }
		}
		if !message.is_empty() {
			msglog.tell_player(message);
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
pub fn item_collection_system(mut commands: Commands,
	                          mut ereader:  EventReader<GameEvent>,
	                          mut msglog:   ResMut<MessageLog>,
	                          // The list of Entities that also have Containers
	                          e_query:      Query<(Entity, &Name, &Position, &Container, Option<&Player>)>,
	                          // The list of every Item that may or may not be in a container
	                          i_query:      Query<(Entity, &Name, &Portable, Option<&Position>)>,
) {
	for event in ereader.iter() {
		if event.etype != ItemMove
		&& event.etype != ItemDrop
		&& event.etype != ItemKILL { continue; }
		if event.context.is_none() { continue; }
		let econtext = event.context.as_ref().unwrap();
		if econtext.is_invalid() { continue; } // TODO: consider renaming this function...
		let mut message: String = "".to_string();
		let subject = e_query.get(econtext.subject).unwrap();
		let subject_name = subject.1.name.clone();
		let player_action = subject.4.is_some();
		let object = i_query.get(econtext.object).unwrap();
		let item_name = object.1.name.clone();
		match event.etype {
			ItemMove => { // Move an Item into an Entity's possession
				commands.entity(object.0)
				.insert(Portable{carrier: subject.0}) // put the container's ID to the target's Portable component
				.remove::<Position>(); // remove the Position component from the target
				// note that the above simply does nothing if it doesn't exist,
				// and inserting a Component that already exists overwrites the previous one,
				// so it's safe to call even on enty -> enty transfers
				if player_action {
					message = format!("Obtained a {}.", item_name);
				} else {
					message = format!("The {} takes a {}.", subject_name, item_name);
				}
			}
			ItemDrop => { // Remove an Item and place it into the World
				let location = subject.2;
				commands.entity(object.0)
				.insert(Portable{carrier: Entity::PLACEHOLDER}) // still portable but not carried
				.insert(Position{x: location.x, y: location.y, z: location.z});
				if player_action {
					message = format!("Dropped a {}.", item_name);
				} else {
					message = format!("The {} drops a {}.", subject_name, item_name);
				}
			}
			ItemKILL => { // DESTROY an Item entirely, ie remove it from the game
				commands.entity(econtext.object).despawn();
			}
			_ => { /* do nothing */ }
		}
		if !message.is_empty() {
			msglog.tell_player(message);
		}
	}
}
/// Allows us to run PLANQ updates and methods in their own thread, just like a real computer~
pub fn planq_system(mut ereader: EventReader<GameEvent>, // subject to change
	                mut planq: ResMut<PlanqSettings>, // contains the PLANQ's settings and data storage
	                p_query: Query<(Entity, &Position), With<Player>>, // provides interface to player data
	                i_query: Query<(Entity, &Portable), Without<Position>>,
) {
	/* TODO: Implement level generation such that the whole layout can be created at startup from a
	 * tree of rooms, rather than by directly loading a REXPaint map; by retaining this tree-list
	 * of rooms in the layout, the PLANQ can then show the player's location as a room name
	 */
	// Update the planq's settings if there are any changes queued up
	let player = p_query.get_single().unwrap();
	for event in ereader.iter() {
		match event.etype {
			PlanqEvent(p_cmd) => {
				match p_cmd {
					Startup => { planq.is_running = true; } // TODO: convert field to planq.state
					Shutdown => { planq.is_running = false; }
					Reboot => { }
					InventoryUse => {
						planq.inventory_toggle(); // display the inventory menu
						planq.action_mode = PlanqActionMode::UseItem;
					}
					InventoryDrop => {
						planq.inventory_toggle(); // display the inventory menu
						planq.action_mode = PlanqActionMode::DropItem;
					}
				}
			}
			_ => { }
		}
	}
	if planq.show_inventory {
		// fill the planq's inventory list
		planq.inventory_list = Vec::new();
		for item in i_query.iter().enumerate() {
			if item.1.1.carrier == player.0 {
				planq.inventory_list.push(item.1.0);
			}
		}
	}
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
