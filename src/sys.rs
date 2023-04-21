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
		Container   { contents: Vec::new() },
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
pub fn engine_system(mut _commands:      Commands,
	                 mut state:         ResMut<GameSettings>,
	                 mut ereader:       EventReader<GameEvent>,
	                 p_query:           Query<(Entity, &Position), With<Player>>,
	                 q_query:           Query<(Entity, &Portable), With<Planq>>,
	                 p_items_query:     Query<(Entity, &Portable), Without<Position>>,
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
	// TODO: the gameover conditions are somewhat protracted, not sure yet on health model
	// Check for the victory state
	let player = p_query.get_single().unwrap();
	let planq = q_query.get_single().unwrap();
	let mut p_inventory = Vec::new();
	for item in p_items_query.iter() {
		if item.1.carrier == player.0 { p_inventory.push(item.0); }
	}
	// version 0.1: Player must be standing in the specified Position
	//if *player.1 == Position::new(28, 1, 1) { state.mode = EngineMode::GoodEnd; } // FIXME: partialeq?
	// version 0.2: v0.1 AND Player.has == planq
	if *player.1 == Position::new(28, 1, 1)
	&& p_inventory.contains(&planq.0)
	{ state.mode = EngineMode::GoodEnd; }
}
/// Handles entities that can move around the map
pub fn movement_system(mut ereader:     EventReader<GameEvent>,
	                   model:           Res<Model>,
	                   mut msglog:      ResMut<MessageLog>,
	                   mut p_posn_res:  ResMut<Position>,
	                   mut p_query:     Query<(&mut Position, &mut Viewshed), With<Player>>,
	                   enty_query:      Query<(&Position, &Name, Option<&mut Viewshed>), Without<Player>>,
) { // NOTE: these Events are custom jobbers, see the GameEvent enum in the components
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
/// Provides a map of blocked tiles, among other things, to the pathfinding systems
pub fn map_indexing_system(_ereader:    EventReader<GameEvent>,
	                       mut model:   ResMut<Model>,
	                       mut blocker_query: Query<&Position, With<Obstructive>>,
	                       _enty_query:  Query<(Entity, &Position)>
) {
	// TODO: consider possible optimization for not updating levels that the player is not on?
	// might require some extra smartness to allow updates if the LMR does something out of sight
	// First, rebuild the blocking map by the map tiles
	let mut f_index = 0;
	for floor in model.levels.iter_mut() {
		floor.update_blocked_tiles();
		for guy in blocker_query.iter_mut() {
			if guy.z != f_index { continue; }
			let index = floor.to_index(guy.x, guy.y);
			floor.blocked_tiles[index] = true;
		}
		// Then, step through all blocking entities and flag their locations on the map as well
		for guy in blocker_query.iter_mut() {
			if guy.z != f_index { continue; }
			let index = floor.to_index(guy.x, guy.y);
			floor.blocked_tiles[index] = true;
		}
		f_index += 1;
	}
}
/// Handles DoorOpen/Close events
pub fn door_system(mut commands:    Commands,
                   mut ereader:     EventReader<GameEvent>,
                   mut door_query:  Query<(Entity, &Position, &mut Openable, &mut Renderable, Option<&Obstructive>)>,
) {
	for event in ereader.iter() {
		if event.context.is_none() { return; }
		let econtext = event.context.as_ref().unwrap();
		match event.etype {
			GameEventType::DoorOpen => {
				for mut door in door_query.iter_mut() {
					if door.0 == econtext.object {
						door.2.is_open = true;
						door.3.glyph = door.2.open_glyph.clone();
						commands.entity(door.0).remove::<Obstructive>();
					}
				}
			}
			GameEventType::DoorClose => {
				for mut door in door_query.iter_mut() {
					if door.0 == econtext.object {
						door.2.is_open = false;
						door.3.glyph = door.2.closed_glyph.clone();
						commands.entity(door.0).insert(Obstructive {});
					}
				}
			}
			_ => { }
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
	                          mut _model:    ResMut<Model>,
	                          mut msglog:   ResMut<MessageLog>,
	                          // The list of every Item that is in a container
	                          i_query:      Query<(Entity, &Name, &Portable, Option<&Position>)>,
	                          // The list of every non-player Container
	                          e_query:      Query<(Entity, &Name, &Position, &Container), Without<Player>>,
	                          // The player
	                          p_query:      Query<(Entity, &Name, &Position, &Container), With<Player>>
) {
	for event in ereader.iter() {
		if event.context.is_none() { return; } // All these actions require context info
		let econtext = event.context.as_ref().unwrap();
		let message: String;
		let item_name = i_query.get(econtext.object).unwrap().1.to_string();
		let player = p_query.get_single().unwrap();
		// assume this was a player action by arbitrary default
		let mut subject = player.0;
		let mut location = player.2;
		let mut subject_name = player.1.name.clone();
		let mut player_action = true;
		if econtext.subject != player.0 { // but in case it wasn't, set the subject accordingly
			for enty in e_query.iter() {
				if enty.0 == econtext.subject {
					subject = enty.0;
					location = enty.2;
					subject_name = enty.1.name.clone();
					player_action = false;
					break; // Entity IDs are guaranteed to be unique, therefore stop at first match
				}
			}
		}
		// Prefer to dispatch the message immediately when it is finished, as not every branch
		// in this logic should actually generate a message (ie ItemKILL)
		match event.etype {
			// An Item is moving from the World into an entity's Container: "pick up"
			// or is moving between possession of entities: "give"
			ItemMove => {
				if player_action {
					message = format!("Obtained a {}.", item_name);
				} else {
					message = format!("The {} takes a {}.", subject_name, item_name);
				}
				msglog.tell_player(message);
				commands.entity(econtext.object)
				.insert(Portable{carrier: subject}) // put the container's ID to the target's Portable component
				.remove::<Position>(); // remove the Position component from the target
				// note that the above simply does nothing if it doesn't exist
				// so it's safe to call on enty -> enty transfers
			}
			// An Item is being dropped from an Entity to the World
			ItemDrop => {
				if player_action {
					message = format!("Dropped a {}.", item_name);
				} else {
					message = format!("The {} drops a {}.", subject_name, item_name);
				}
				msglog.tell_player(message);
				commands.entity(econtext.object)
				.insert(Portable{carrier: Entity::PLACEHOLDER}) // still portable but not carried
				.insert(Position{x: location.x, y: location.y, z: location.z});
			}
			// Permanently removes an Item from the game
			ItemKILL => {
				commands.entity(econtext.object).despawn();
			}
			// NOTE: this system does not (yet?) handle item creation requests
			_ => { }
		}
	}
}
/// Allows us to run PLANQ updates and methods in their own thread, just like a real computer~
pub fn planq_system(mut ereader: EventReader<GameEvent>, // subject to change
	                p_query: Query<(Entity, &Position), With<Player>>, // provides interface to player data
	                mut planq: ResMut<PlanqSettings>, // contains the PLANQ's settings and data storage
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
