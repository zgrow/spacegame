// sys.rs
// July 12 2023

// Disable some of the more irritating clippy warnings
#![allow(clippy::type_complexity)]
#![allow(clippy::single_match)]
#![allow(clippy::needless_lifetimes)]

// *** EXTERNAL LIBS
use bevy::ecs::archetype::Archetypes;
use bevy::ecs::component::{ComponentId, Components};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{EventReader, EventWriter};
use bevy::ecs::query::{
	Changed,
	With,
	Without,
};
use bevy::ecs::system::{
	Commands,
	Query,
	Res,
	ResMut
};
use bevy::utils::{Duration, HashSet};
use bevy_turborand::*;
use bracket_pathfinding::prelude::*;
use simplelog::*;

// *** INTERNAL LIBS
use crate::components::*;
use crate::components::{
	Direction,
	Mobile,
	Player,
	Position,
	Renderable
};
use crate::engine::event::*;
use crate::engine::event::GameEventType::*;
use crate::engine::event::ActionType::*;
use crate::engine::messagelog::*;
use crate::engine::planq::*;
use crate::map::*;

// *** CONTINUOUS SYSTEMS
/// Handles connections between maintenance devices like the PLANQ and access ports on external entities
pub fn access_port_system(mut ereader:      EventReader<GameEvent>,
	                        mut preader:      EventWriter<PlanqEvent>,
	                        mut msglog:       ResMut<MessageLog>,
	                        mut planq:        ResMut<PlanqData>,
	                        a_query:          Query<(Entity, &Description), With<AccessPort>>,
) {
	for event in ereader.iter() {
		match event.etype {
			GameEventType::PlanqConnect(Entity::PLACEHOLDER) => {
				planq.jack_cnxn = Entity::PLACEHOLDER;
				let object_name = a_query.get(planq.jack_cnxn).unwrap().1;
				msglog.tell_player(format!("The PLANQ's access jack unsnaps from the {}.", object_name));
				preader.send(PlanqEvent::new(PlanqEventType::AccessUnlink))
			}
			GameEventType::PlanqConnect(target) => {
				planq.jack_cnxn = event.context.unwrap().object;
				msglog.tell_player(format!("The PLANQ's access jack clicks into place on the {:?}.", target));
				preader.send(PlanqEvent::new(PlanqEventType::AccessLink))
			}
			_ => { }
		}
	}
}
/// Maintains accurate ActionSets on Entities, among other future things
pub fn action_referee_system(_cmd:       Commands, // gonna need this eventually if i want to despawn entys
	                           archetypes:    &Archetypes,
	                           components:    &Components,
	                           mut a_query:   Query<(Entity, &mut ActionSet), Changed<ActionSet>>,
) {
	for mut actor in a_query.iter_mut() {
		if actor.1.outdated {
			info!("* Running update on an ActionSet..."); // DEBUG: announce ActionSet update
			let mut new_set = HashSet::new();
			for comp_id in get_components_for_entity(actor.0, archetypes).unwrap() {
				if let Some(comp_info) = components.get_info(comp_id) {
					let split_str: Vec<&str> = comp_info.name().split("::").collect();
					let comp_name = split_str[split_str.len() - 1];
					match comp_name {
						"Description" => { new_set.insert(ActionType::Examine); }
						"Portable"    => {
							new_set.insert(ActionType::MoveItem);
							new_set.insert(ActionType::DropItem);
						}
						"Openable"    => {
							new_set.insert(ActionType::OpenItem);
							new_set.insert(ActionType::CloseItem);
						}
						"Lockable"    => {
							new_set.insert(ActionType::UnlockItem);
							new_set.insert(ActionType::LockItem);
						}
						"Key"         => {
							new_set.insert(ActionType::UnlockItem);
							new_set.insert(ActionType::LockItem);
						}
						"Device"      => {
							new_set.insert(ActionType::UseItem);
						}
						_ => { }
					}
				}
			}
			debug!("* {:?}", new_set); // DEBUG: display the newly made action set
			actor.1.actions = new_set;
			actor.1.outdated = false;
		}
	}
}
/// Handles requests for descriptions of entities by the player
pub fn examination_system(mut ereader:  EventReader<GameEvent>,
	                        mut msglog:   ResMut<MessageLog>,
	                        e_query:      Query<(Entity, &Description)>,
) {
	if ereader.is_empty() { return; }
	for event in ereader.iter() {
		if event.etype != PlayerAction(ActionType::Examine) { continue; }
		let econtext = event.context.as_ref().unwrap();
		if econtext.object == Entity::PLACEHOLDER {
			warn!("* Attempted to Examine the Entity::PLACEHOLDER"); // DEBUG: warn if this case occurs
			continue;
		}
		let output_ref = e_query.get(econtext.object).unwrap();
		let output = output_ref.1.desc.clone();
		msglog.tell_player(output);
	}
}
/// Handles pickup/drop/destroy requests for Items
pub fn item_collection_system(mut cmd:      Commands,
	                            mut ereader:  EventReader<GameEvent>,
	                            mut msglog:   ResMut<MessageLog>,
	                            // The list of Entities that also have Containers
	                            e_query:      Query<(Entity, &Description, &Position, &Container, Option<&Player>)>,
	                            // The list of every Item that may or may not be in a container
	                            i_query:      Query<(Entity, &Description, &Portable, Option<&Position>)>,
) {
	// Don't even bother trying if there's no events to worry about
	if ereader.is_empty() { return; }
	for event in ereader.iter() {
		// Skip any events with the wrong type
		let atype: ActionType;
		match event.etype {
			PlayerAction(action) | ActorAction(action) => {
				match action {
					ActionType::MoveItem
					| ActionType::DropItem
					| ActionType::KillItem => { atype = action; }
					_ => { continue; }
				}
			}
			_ => { continue; }
		};
		// All of the item events require an event context, so if there isn't any then don't try to handle the event
		if event.context.is_none() { continue; }
		let econtext = event.context.as_ref().unwrap();
		// We know that it is safe to unwrap these because calling is_invalid() checked that they are not placeholders
		let subject = e_query.get(econtext.subject).unwrap();
		let subject_name = subject.1.name.clone();
		let is_player_action = subject.4.is_some();
		let object = i_query.get(econtext.object).unwrap();
		let item_name = object.1.name.clone();
		// We have all of our context values now, so proceed to actually doing the requested action
		let mut message: String = "".to_string();
		match atype {
			ActionType::MoveItem => { // Move an Item into an Entity's possession
				debug!("* Moving item..."); // DEBUG: announce item movement
				cmd.entity(object.0)
				.insert(Portable{carrier: subject.0}) // put the container's ID to the target's Portable component
				.remove::<Position>(); // remove the Position component from the target
				// Note that the above simply does nothing if it doesn't exist,
				//   and inserting a Component that already exists overwrites the previous one,
				//   so it's safe to call even on enty -> enty transfers
				if is_player_action {
					message = format!("Obtained a {}.", item_name);
				} else {
					message = format!("The {} takes a {}.", subject_name, item_name);
				}
			}
			ActionType::DropItem => { // Remove an Item and place it into the World
				debug!("* Dropping item..."); // DEBUG: announce item drop
				let location = subject.2;
				cmd.entity(object.0)
				.insert(Portable{carrier: Entity::PLACEHOLDER}) // still portable but not carried
				.insert(Position{x: location.x, y: location.y, z: location.z});
				if is_player_action {
					message = format!("Dropped a {}.", item_name);
				} else {
					message = format!("The {} drops a {}.", subject_name, item_name);
				}
			}
			ActionType::KillItem => { // DESTROY an Item entirely, ie remove it from the game
				debug!("* KILLing item..."); // DEBUG: announce item destruction
				cmd.entity(econtext.object).despawn();
			}
			action => {
				error!("* item_collection_system unhandled action: {}", action); // DEBUG: announce unhandled action for this item
			}
		}
		if !message.is_empty() {
			msglog.add(message, "world".to_string(), 0, 0);
		}
	}
}
/// Handles ActorLock/Unlock events
pub fn lockable_system(mut _commands:    Commands,
	                     mut ereader:      EventReader<GameEvent>,
	                     mut msglog:       ResMut<MessageLog>,
	                     mut lock_query:   Query<(Entity, &Position, &Description, &mut Lockable)>,
	                     mut e_query:      Query<(Entity, &Position, &Description, Option<&Player>)>,
	                     key_query:        Query<(Entity, &Portable, &Description, &Key), Without<Position>>,
) {
	if ereader.is_empty() { return; }
	for event in ereader.iter() {
		let mut atype = ActionType::NoAction;
		if let PlayerAction(action) | ActorAction(action) = event.etype {
			if action != LockItem && action != UnlockItem {
				continue;
			} else {
				atype = action;
			}
		}
		if event.context.is_none() { continue; }
		let econtext = event.context.as_ref().unwrap();
		let actor = e_query.get_mut(econtext.subject).unwrap();
		let player_action = actor.3.is_some();
		let mut target = lock_query.get_mut(econtext.object).unwrap();
		let mut message: String = "".to_string();
		match atype {
			ActionType::LockItem => {
				target.3.is_locked = true;
				if player_action {
					message = format!("You tap the LOCK button on the {}.", target.2.name.clone());
				} else {
					message = format!("The {} locks the {}.", actor.2.name.clone(), target.2.name.clone());
				}
			}
			ActionType::UnlockItem => {
				// Obtain the set of keys that the actor is carrying
				let mut carried_keys: Vec<(Entity, i32, String)> = Vec::new();
				for key in key_query.iter() {
					if key.1.carrier == actor.0 { carried_keys.push((key.0, key.3.key_id, key.2.name.clone())); }
				}
				if carried_keys.is_empty() { continue; } // no keys to try!
				// The actor has at least one key to try in the lock
				for key in carried_keys.iter() {
					if key.1 == target.3.key {
						// the subject has the right key, unlock the lock
						target.3.is_locked = false;
						if player_action {
							message = format!("Your {} unlocks the {}.", key.2, target.2.name.clone());
						} else {
							message = format!("The {} unlocks the {}.", actor.2.name.clone(), target.2.name.clone());
						}
					} else {
						// none of the keys worked, report a failure
						if player_action {
							message = "You don't seem to have the right key.".to_string();
						}
					}
				}
			}
			_ => { }
		}
		if !message.is_empty() {
			msglog.tell_player(message);
		}
	}
}
/// Handles updates to the 'meta' worldmaps, ie the blocked and opaque tilemaps
pub fn map_indexing_system(mut model:         ResMut<Model>,
	                         blocker_query: Query<&Position, With<Obstructive>>,
	                         opaque_query:  Query<(&Position, &Opaque)>,
) {
	// First, rebuild the blocking map by the map tiles
	let mut f_index = 0;
	let mut index;
	for floor in model.levels.iter_mut() {
		floor.update_tilemaps(); // Update tilemaps based on their tiletypes
		// Then, step through all blocking entities and flag their locations on the map as well
		for guy in blocker_query.iter() {
			if guy.z != f_index { continue; }
			index = floor.to_index(guy.x, guy.y);
			floor.blocked_tiles[index] = true;
		}
		// Do the same for the opaque entities
		for guy in opaque_query.iter() {
			if guy.0.z != f_index { continue; }
			index = floor.to_index(guy.0.x, guy.0.y);
			floor.opaque_tiles[index] = guy.1.opaque;
		}
		f_index += 1;
	}
}
/// Handles updates for entities that can move around
pub fn movement_system(mut ereader:     EventReader<GameEvent>,
	                     mut msglog:      ResMut<MessageLog>,
	                     mut p_posn_res:  ResMut<Position>,
	                     mut model:           ResMut<Model>,
	                     mut e_query:     Query<(Entity, &Description, &mut Position, Option<&mut Viewshed>)>
) {
	if ereader.is_empty() { return; } // Don't even bother trying if there's no events to worry about
	for event in ereader.iter() {
		// Only process the event if it's an ____Action(MoveTo(dir)) type
		if let PlayerAction(atype) | ActorAction(atype) = event.etype {
			if let MoveTo(dir) = atype {
				let is_player_action = same_enum_variant(&event.etype, &PlayerAction(NoAction));
				if event.context.is_none() {
					error!("* ! no context for actor movement"); // DEBUG: warn if the actor's movement is broken
					continue;
				}
				let econtext = event.context.unwrap();
				let origin = e_query.get_mut(econtext.subject);
				let (_, _, mut actor_posn, view_ref) = origin.unwrap();
				let mut xdiff = 0;
				let mut ydiff = 0;
				let mut zdiff = 0; // NOTE: not a typical component: z-level indexes to map stack, not Euclidean space
				match dir { // Calculate the offsets required from the specified direction
					Direction::X    => { }
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
				let mut new_location = Position::create(actor_posn.x + xdiff, actor_posn.y + ydiff, actor_posn.z + zdiff);
				if dir == Direction::UP || dir == Direction::DOWN { // Is the actor moving between z-levels?
					// Prevent movement if an invalid z-level was calculated, or if they are not standing on stairs
					if new_location.z < 0 || new_location.z as usize >= model.levels.len() {
						msglog.tell_player(format!("There's no way to go {} from here.", dir));
						continue;
					}
					let actor_index = model.levels[actor_posn.z as usize].to_index(actor_posn.x, actor_posn.y);
					if model.levels[actor_posn.z as usize].tiles[actor_index].ttype != TileType::Stairway {
						msglog.tell_player(format!("You're not standing on anything that allows you to go {}.", dir));
						continue;
					}
					let possible = model.get_exit(*actor_posn);
					if let Some(portal) = possible {
						new_location = portal;
					}
				}
				let locn_index = model.levels[new_location.z as usize].to_index(new_location.x, new_location.y);
				if model.levels[new_location.z as usize].blocked_tiles[locn_index] { // Is there anything blocking movement?
					// CASE 1: Another Actor is blocking the way
					for guy in e_query.iter() {
						if guy.2 == &new_location {
							msglog.tell_player(format!("The way {} is blocked by the {}.", dir, guy.1));
							return;
						}
					}
					// CASE 2: An inert Entity (ie a Thing or Fixture) is blocking the way
					msglog.tell_player(format!("The way {} is blocked by a {}.",
						                 dir, &model.levels[new_location.z as usize].tiles[locn_index].ttype.to_string()));
					return;
				}
				if let Some(mut viewshed) = view_ref { // If the actor has a Viewshed, flag it as dirty to be updated
					viewshed.dirty = true;
				}
				*actor_posn = new_location; // Nothing's in the way, so go ahead and update the actor's position
				if is_player_action { // Was it the player that's moving around?
					// Is there anything on the ground at the new location?
					*p_posn_res = new_location; // Update the system-wide resource containing the player's location
					let mut contents = Vec::new();
					for enty in e_query.iter() {
						if *enty.2 == new_location {
							contents.push(&enty.1.name);
						}
					}
					// If so, tell the player about it
					if !contents.is_empty() {
						let message = if contents.len() <= 3 { // Make a shortlist if there's only a couple items here
							let mut text = "There's a ".to_string();
							loop {
								text.push_str(contents.pop().unwrap());
								if contents.is_empty() { break; }
								else { text.push_str(", and a "); }
							}
							text.push_str(" here.");
							text
						} else { // Just summarize since there's more there than we can list in one line
							"There's some stuff here on the ground.".to_string()
						};
						msglog.tell_player(message);
					}
				}
			}
		}
	}
}
/// Handles updates for entities that can open and close
pub fn openable_system(mut commands:    Commands,
	                     mut ereader:     EventReader<GameEvent>,
	                     mut msglog:      ResMut<MessageLog>,
	                     mut door_query:  Query<(Entity, &Position, &mut Openable, &mut Renderable, &mut Opaque, Option<&Obstructive>)>,
	                     mut e_query:     Query<(Entity, &Position, &Description, Option<&Player>, Option<&mut Viewshed>)>,
) {
	if ereader.is_empty() { return; }
	for event in ereader.iter() {
		let mut atype = ActionType::NoAction;
		if let PlayerAction(action) | ActorAction(action) = event.etype {
			if action != OpenItem && action != CloseItem {
				continue;
			} else {
				atype = action;
			}
		}
		if event.context.is_none() { continue; }
		let econtext = event.context.as_ref().unwrap();
		debug!("actor opening door {0:?}", econtext.object); // DEBUG: announce opening door
		let actor = e_query.get_mut(econtext.subject).unwrap();
		let player_action = actor.3.is_some();
		let mut message: String = "".to_string();
		match atype {
			ActionType::OpenItem => {
				debug!("Trying to open a door"); // DEBUG: announce opening a door
				for mut door in door_query.iter_mut() {
					if door.0 == econtext.object {
						door.2.is_open = true;
						door.3.glyph = door.2.open_glyph.clone();
						door.4.opaque = false;
						commands.entity(door.0).remove::<Obstructive>();
					}
				}
				if player_action {
					message = "You open the [door]".to_string();
				} else {
					message = format!("The {} opens a [door].", actor.2.name.clone());
				}
				if actor.4.is_some() { actor.4.unwrap().dirty = true; }
			}
			ActionType::CloseItem => {
				debug!("Trying to close a door"); // DEBUG: announce closing door
				for mut door in door_query.iter_mut() {
					if door.0 == econtext.object {
						door.2.is_open = false;
						door.3.glyph = door.2.closed_glyph.clone();
						door.4.opaque = true;
						commands.entity(door.0).insert(Obstructive {});
					}
				}
				if player_action {
					message = "The [door] slides shut.".to_string();
				} else {
					message = format!("The {} closes a [door].", actor.2.name.clone());
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
/// Handles anything related to the CanOperate component: ActorUse, ToggleSwitch, &c
pub fn operable_system(mut ereader: EventReader<GameEvent>,
                       //mut o_query: Query<(Entity, &Position, &Name), With<CanOperate>>,
                       mut d_query: Query<(Entity, &Description, &mut Device)>,
) {
	if ereader.is_empty() { return; }
	for event in ereader.iter() {
		if let PlayerAction(action) | ActorAction(action) = event.etype {
			if action != UseItem {
				continue;
			}
		}
		let econtext = event.context.as_ref().unwrap();
		if econtext.is_blank() { continue; }
		//let operator = o_query.get(econtext.subject).unwrap();
		let mut device = d_query.get_mut(econtext.object).unwrap();
		if !device.2.pw_switch { // If it's not powered on, assume that function first
			device.2.power_toggle();
		}
		// TODO: there's definitely going to be more stuff to implement here depending on the actual Device
	}
}
/// Handles entities that can see physical light
pub fn visibility_system(mut model:  ResMut<Model>,
	                       mut seers:  Query<(&mut Viewshed, &Position, Option<&Player>, Option<&mut Memory>), Changed<Viewshed>>,
	                       observable: Query<(Entity, &Position, &Renderable)>,
) {
	for (mut viewshed, s_posn, player, memory) in &mut seers {
		debug!("* [vis_sys] s_posn: {s_posn:?}"); // DEBUG: print the position of the entity being examined
		if viewshed.dirty {
			assert!(s_posn.z != -1);
			let map = &mut model.levels[s_posn.z as usize];
			viewshed.visible_tiles.clear();
			viewshed.visible_tiles = field_of_view(posn_to_point(s_posn), viewshed.range, map);
			viewshed.visible_tiles.retain(|p| p.x >= 0 && p.x < map.width
				                             && p.y >= 0 && p.y < map.height
			);
			if let Some(_player) = player { // if this is the player...
				for s_posn in &viewshed.visible_tiles { // For all the player's visible tiles...
					// ... set the corresponding tile in the map.revealed_tiles to TRUE
					let map_index = map.to_index(s_posn.x, s_posn.y);
					map.revealed_tiles[map_index] = true;
				}
			}
			if let Some(mut recall) = memory { // If the seer entity has a memory...
				for v_posn in &viewshed.visible_tiles { // Iterate on all tiles they can see:
					let new_posn = Position::create(v_posn.x, v_posn.y, s_posn.z);
					for target in observable.iter() {
						if *target.1 == new_posn {
							recall.visual.insert(target.0, new_posn);
						}
					}
				}
			}
			viewshed.dirty = false;
		}
	}
}

// *** SINGLETON SYSTEMS
/// Adds a new player entity to a new game world
pub fn new_player_spawn(mut commands: Commands,
	                      spawnpoint:   Res<Position>,
	                      mut p_query:  Query<(Entity, &Player)>,
	                      mut msglog:   ResMut<MessageLog>,
	                      mut global_rng: ResMut<GlobalRng>,
) {
	if !p_query.is_empty() {
		info!("* Existing player found, treating as a loaded game"); // DEBUG: announce possible game load
		let player = p_query.get_single_mut().unwrap();
		commands.entity(player.0).insert(Viewshed::new(8));
		return;
	}
	let player = commands.spawn((
		Player { },
		ActionSet::new(),
		Description::new("Pleyeur".to_string(), "Still your old self.".to_string()),
		*spawnpoint,
		Renderable::new().glyph("@".to_string()).fg(2).bg(0),
		Viewshed::new(8),
		Mobile::default(),
		Obstructive::default(),
		Container::default(),
		Memory::new(),
	)).id();
	debug!("* new_player_spawn spawned @{spawnpoint:?}"); // DEBUG: print spawn location of new player
	commands.spawn((
		Planq::new(),
		ActionSet::new(),
		Description::new("PLANQ".to_string(), "It's your PLANQ.".to_string()),
		Renderable::new().glyph("¶".to_string()).fg(3).bg(0),
		Portable::new(player),
		Device::new(-1),
		RngComponent::from(&mut global_rng),
	));
	debug!("* new planq spawned into player inventory"); // DEBUG: announce creation of player's planq
	commands.spawn(DataSampleTimer::new().source("current_time".to_string()));
	commands.spawn(DataSampleTimer::new().source("planq_battery".to_string()));
	commands.spawn(DataSampleTimer::new().source("planq_mode".to_string()));
	msglog.tell_player("[[fg:green]]WELCOME[[end]] TO [[fg:blue,mod:+italic]]SPACEGAME[[end]]".to_string());
}
/// Spawns a new LMR at the specified Position, using default values
pub fn new_lmr_spawn(mut commands:  Commands,
	                   mut msglog:    ResMut<MessageLog>,
) {
	commands.spawn((
		LMR         { },
		ActionSet::new(),
		Description::new("LMR".to_string(), "The Light Maintenance Robot is awaiting instructions.".to_string()),
		Position::create(12, 12, 0), // TODO: remove magic numbers
		Renderable::new().glyph("l".to_string()).fg(14).bg(0),
		Viewshed::new(5),
		Mobile::default(),
		Obstructive::default(),
		Container::default(),
		Opaque::new(true),
	));
	msglog.add(format!("LMR spawned at {}, {}, {}", 12, 12, 0), "debug".to_string(), 1, 1);
}
/// Adds a demo NPC to the game world
pub fn test_npc_spawn(mut commands: Commands,
	                    mut rng:      ResMut<GlobalRng>,
	                    e_query:      Query<(Entity, &Position)>,
) {
	let spawnpoint = Position::create(rng.i32(1..30), rng.i32(1..30), 0);
	// Check the spawnpoint for collisions
	loop {
		let mut found_open_tile = true;
		for enty in e_query.iter() { // FIXME: this should probably be a call to Model.is_occupied instead
			if enty.1 == &spawnpoint { found_open_tile = false; }
		}
		if found_open_tile { break; }
	}
	commands.spawn((
		ActionSet::new(),
		Description::new("Jenaryk".to_string(), "Behold, a generic virtual cariacature of a man.".to_string()),
		spawnpoint,
		Renderable::new().glyph("&".to_string()).fg(1).bg(0),
		Viewshed::new(8),
		Mobile::default(),
		Obstructive::default(),
		Container::default(),
	));
	debug!("* Spawned new npc at {}", spawnpoint); // DEBUG: announce npc creation
}

// *** UTILITIES
/// Converts my Position type into a bracket_pathfinding::Point
pub fn posn_to_point(input: &Position) -> Point { Point { x: input.x, y: input.y } }
/// If the Entity exists, will return an Iterator that contains info on all the Components that belong to that Entity
/// rust-clippy insists that the lifetime annotation here is useless, however!
/// Removing the annotation causes errors, because there is a *hidden type* that *does* capture a lifetime parameter
/// Not sure how to get clippy to not report a false-positive, but this code is 100% known to work, i've tested it
pub fn get_components_for_entity<'a>(entity: Entity,
	                                   archetypes: &'a Archetypes
) -> Option<impl Iterator<Item=ComponentId> + 'a> {
	for archetype in archetypes.iter() {
		if archetype.entities().iter().any(|x| x.entity() == entity) {
			return Some(archetype.components());
		}
	}
	None
}
/// This is a lil reverse-trait/extension trait that provides some shorthand for the Duration type provided by Bevy
/// Defining a trait on an external type like this allows the trait methods to be called on instances of the type as self
/// Note that this does not change any of the scope hierarchy; the only methods callable here are the public methods defined
/// by the Display type
/// The concept has two parts:
/// 1) Define a new trait with the signatures of the desired methods
/// 2) Implement the new trait T on the external type Y: 'impl T for Y { ... }'
/// source: http://xion.io/post/code/rust-extension-traits.html
pub trait DurationFmtExt {
	fn get_as_string(self) -> String;
	fn get_as_msecs(self) -> u128;
}
impl DurationFmtExt for Duration {
	/// Provides the time as a preformatted string, suitable for display.
	fn get_as_string(self) -> String {
		let mut secs = self.as_secs();
		let mils = self.subsec_millis();
		let hours: u64 = secs / 3600;
		secs -= hours * 3600;
		let mins: u64 = secs / 60;
		secs -= mins * 60;
		format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, mils)
	}
	/// Provides the current ship time as a raw quantity of milliseconds, suitable for doing maths to.
	fn get_as_msecs(self) -> u128 {
		self.as_millis()
	}
}

// EOF
