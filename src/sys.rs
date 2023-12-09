// sys.rs
// July 12 2023

// Disable some of the more irritating clippy warnings
#![allow(clippy::type_complexity)]
#![allow(clippy::single_match)]
#![allow(clippy::needless_lifetimes)]

// ###: EXTERNAL LIBS
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

// ###: INTERNAL LIBS
use crate::camera::*;
use crate::components::*;
use crate::components::{
	Direction,
	Mobile,
	Player,
	Position,
};
use crate::engine::event::*;
use crate::engine::event::GameEventType::*;
use crate::engine::event::ActionType::*;
use crate::engine::messagelog::*;
use crate::engine::planq::*;
use crate::worldmap::*;

// ###: CONTINUOUS SYSTEMS
/// Handles connections between maintenance devices like the PLANQ and access ports on external entities
pub fn access_port_system(mut ereader:      EventReader<GameEvent>,
	                        mut preader:      EventWriter<PlanqEvent>,
	                        mut msglog:       ResMut<MessageLog>,
	                        mut planq:        ResMut<PlanqData>,
	                        a_query:          Query<(Entity, &Description), With<AccessPort>>,
) {
	// For every event in the Game's event queue,
	//   Assign the planq's jack connection to the target entity,
	//   Send a feedback message to the player to inform them of the change,
	//   Send an appropriate PLANQ event to the queue
	for event in ereader.iter() {
		match event.etype {
			GameEventType::PlanqConnect(Entity::PLACEHOLDER) => {
				planq.jack_cnxn = Entity::PLACEHOLDER;
				if let Ok((_enty, object_name)) = a_query.get(planq.jack_cnxn) {
					msglog.tell_player(format!("The PLANQ's access jack unsnaps from the {}.", object_name).as_str());
					preader.send(PlanqEvent::new(PlanqEventType::AccessUnlink))
				}
			}
			GameEventType::PlanqConnect(target) => {
				if let Some(context) = event.context {
					planq.jack_cnxn = context.object;
					msglog.tell_player(format!("The PLANQ's access jack clicks into place on the {:?}.", target).as_str());
					preader.send(PlanqEvent::new(PlanqEventType::AccessLink))
				}
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
	// For every actor whose ActionSet has changed, (implying that ActionSet.outdated changed values)
	//   If that really is what changed (to avoid triggering recursively on subsequent system cycles):
	//     Get a stringified list of the Components that form the Entity,
	//     Clean up the strings,
	//     Match the strings to commands that will add the correct ActionTypes to the ActionSet Component
	//     Update the actor's ActionSet component
	//     Set the ActionSet.outdated flag to false to avoid double-updates
	for (a_enty, mut a_actionset) in a_query.iter_mut() { // Use tuple indexing instead of destructive binding
		if a_actionset.outdated {
			if let Some(component_iter) = get_components_for_entity(a_enty, archetypes) {
				let mut new_set = HashSet::new();
				for comp_id in component_iter {
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
				a_actionset.actions = new_set;
				a_actionset.outdated = false;
			}
		}
	}
}
/// Handles requests for descriptions of entities by the player
pub fn examination_system(mut ereader:  EventReader<GameEvent>,
	                        mut msglog:   ResMut<MessageLog>,
	                        e_query:      Query<(Entity, &Description)>,
) {
	// Bail out if there's no events in the queue
	// For every event in the queue,
	//   Get the target of the EXAMINE action,
	//   Get the target's description,
	//   Show the description to the player
	if ereader.is_empty() { return; }
	for event in ereader.iter() {
		if event.etype != PlayerAction(ActionType::Examine) { continue; }
		if let Some(econtext) = event.context.as_ref() {
			if econtext.object == Entity::PLACEHOLDER {
				warn!("* Attempted to Examine the Entity::PLACEHOLDER"); // DEBUG: warn if this case occurs
				continue;
			}
			if let Ok((_enty, e_desc)) = e_query.get(econtext.object) {
				//let output = e_desc.desc.clone();
				let output = &e_desc.desc;
				msglog.tell_player(output);
			}
		}
	}
}
/// Handles pickup/drop/destroy requests for Items
pub fn item_collection_system(mut cmd:      Commands,
	                            mut ereader:  EventReader<GameEvent>,
	                            mut msglog:   ResMut<MessageLog>,
	                            // The list of Entities that also have Containers
	                            e_query:      Query<(Entity, &Description, &Body, &Container, Option<&Player>)>,
	                            // The list of every Item that may or may not be in a container
	                            mut i_query:      Query<(Entity, &Description, &mut Body, &Portable), Without<Container>>,
) {
	// Don't even bother trying if there's no events to worry about
	if ereader.is_empty() { return; }
	for event in ereader.iter() {
		// Skip any events with the wrong type by filtering on the event's type's action's type
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
		let econtext = event.context.as_ref().expect("event.context should be Some(n)");
		// We know that it is safe to unwrap these because calling is_invalid() checked that they are not placeholders
		//let subject = e_query.get(econtext.subject).expect("econtext.subject should be Some(n)");
		let (s_enty, s_desc, s_body, _container, s_player) = e_query.get(econtext.subject).expect("econtext.subject should be Some(n)");
		let subject_name = s_desc.name.clone();
		let is_player_action = s_player.is_some();
		let (o_enty, o_desc, mut o_body, _) = i_query.get_mut(econtext.object).expect("econtext.object should be Some(n)");
		let item_name = o_desc.name.clone();
		// We have all of our context values now, so proceed to actually doing the requested action
		let mut message: String = "".to_string();
		match atype {
			ActionType::MoveItem => { // Move an Item into an Entity's possession
				// NOTE: the insert(Portable) call below will overwrite any previous instance of that component
				cmd.entity(o_enty)
				.insert(Portable{carrier: s_enty}) // put the container's ID to the target's Portable component
				.insert(IsCarried::default()); // add the IsCarried tag to the component
				if is_player_action {
					message = format!("Obtained a {}.", item_name);
				} else {
					message = format!("The {} takes a {}.", subject_name, item_name);
				}
			}
			ActionType::DropItem => { // Remove an Item and place it into the World
				//debug!("* Dropping item..."); // DEBUG: announce item drop
				cmd.entity(o_enty)
				.insert(Portable{carrier: Entity::PLACEHOLDER}) // still portable but not carried
				.remove::<IsCarried>(); // remove the tag from the component
				o_body.move_to(s_body.ref_posn);
				if is_player_action {
					message = format!("Dropped a {}.", item_name);
				} else {
					message = format!("The {} drops a {}.", subject_name, item_name);
				}
			}
			ActionType::KillItem => { // DESTROY an Item entirely, ie remove it from the game
				//debug!("* KILLing item..."); // DEBUG: announce item destruction
				cmd.entity(o_enty).despawn();
			}
			action => {
				error!("* item_collection_system unhandled action: {}", action); // DEBUG: announce unhandled action for this item
			}
		}
		if !message.is_empty() {
			msglog.add(&message, "world", 0, 0);
		}
	}
}
/// Handles ActorLock/Unlock events
pub fn lockable_system(mut _commands:    Commands,
	                     mut ereader:      EventReader<GameEvent>,
	                     mut msglog:       ResMut<MessageLog>,
	                     mut lock_query:   Query<(Entity, &Body, &Description, &mut Lockable)>,
	                     mut e_query:      Query<(Entity, &Body, &Description, Option<&Player>)>,
	                     key_query:        Query<(Entity, &Portable, &Description, &Key), With<IsCarried>>,
) {
	// Bail out if there's no events or the wrong type
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
		let econtext = event.context.as_ref().expect("event.context should be Some(n)");
		let (e_enty, _body, e_desc, e_player) = e_query.get_mut(econtext.subject).expect("econtext.subject should be found in e_query");
		let player_action = e_player.is_some();
		let (_enty, _portable, l_desc, mut l_lock) = lock_query.get_mut(econtext.object).expect("econtext.object should be found in lock_query");
		let mut message: String = "".to_string();
		// If they have the right key then they can unlock it
		// Lock attempts always succeed
		match atype {
			ActionType::LockItem => {
				l_lock.is_locked = true;
				if player_action {
					message = format!("You tap the LOCK button on the {}.", l_desc.name.clone());
				} else {
					message = format!("The {} locks the {}.", e_desc.name.clone(), l_desc.name.clone());
				}
			}
			ActionType::UnlockItem => {
				// Obtain the set of keys that the actor is carrying
				let mut carried_keys: Vec<(Entity, i32, String)> = Vec::new();
				for (k_enty, k_portable, k_desc, k_key) in key_query.iter() {
					if k_portable.carrier == e_enty { carried_keys.push((k_enty, k_key.key_id, k_desc.name.clone())); }
				}
				if carried_keys.is_empty() { continue; } // no keys to try!
				// The actor has at least one key to try in the lock
				for (_enty, try_key_id, try_key_name) in carried_keys.iter() {
					if *try_key_id == l_lock.key_id {
						// the subject has the right key, unlock the lock
						l_lock.is_locked = false;
						if player_action {
							message = format!("Your {} unlocks the {}.", try_key_name, l_desc.name.clone());
						} else {
							message = format!("The {} unlocks the {}.", e_desc.name.clone(), l_desc.name.clone());
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
			msglog.tell_player(&message);
		}
	}
}
/// Handles updates to the 'meta' worldmaps, ie the blocked and opaque tilemaps
pub fn map_indexing_system(mut model:         ResMut<WorldModel>,
	                         blocker_query: Query<&Body, With<Obstructive>>,
	                         opaque_query:  Query<(&Body, &Opaque)>,
) {
	// Rebuild each map floor-by-floor
	for floor in model.levels.iter_mut() {
		floor.update_tilemaps(); // Update tilemaps based on their tiletypes
	}
	// Then, step through all blocking entities and flag their locations on the map as well
	for guy in blocker_query.iter() {
		for posn in &guy.extent {
			model.set_blocked_state(posn.posn, true);
		}
	}
	// Do the same for the opaque entities
	for guy in opaque_query.iter() {
		for posn in &guy.0.extent {
			model.set_opaque_state(posn.posn, guy.1.opaque);
		}
	}
}
/// Handles updates for entities that can move around
pub fn movement_system(mut ereader:     EventReader<GameEvent>,
	                     mut msglog:      ResMut<MessageLog>,
	                     mut p_posn_res:  ResMut<Position>,
	                     mut model:       ResMut<WorldModel>,
	                     mut e_query:     Query<(Entity, &mut Description, &mut Body, Option<&mut Viewshed>, Option<&Player>)>
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
				let econtext = event.context.expect("event.context should be Some(n)");
				let origin = e_query.get_mut(econtext.subject);
				let (actor_enty, mut actor_desc, mut actor_body, actor_viewshed, _) = origin.expect("econtext.subject should be in e_query");
				// TODO: this is now overkill, just use the match case to make an implicit PosnOffset applied to the old position
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
				let mut new_location = Position::new(actor_body.ref_posn.x + xdiff, actor_body.ref_posn.y + ydiff, actor_body.ref_posn.z + zdiff);
				// If the actor is moving between z-levels, we have some extra logic to handle
				if dir == Direction::UP || dir == Direction::DOWN { // Is the actor moving between z-levels?
					// Prevent movement if an invalid z-level was calculated, or if they are not standing on stairs
					//debug!("* Attempting ladder traverse to target posn {}", new_location);
					// CASE 1: The target location is beyond the Model's height
					if new_location.z < 0 || new_location.z as usize >= model.levels.len() {
						msglog.tell_player(format!("You're already on the {}-most deck.", dir).as_str());
						continue;
					}
					// CASE 2: The actor is not standing on a ladder Tile
					let actor_index = model.levels[actor_body.ref_posn.z as usize].to_index(actor_body.ref_posn.x, actor_body.ref_posn.y);
					if model.levels[actor_body.ref_posn.z as usize].tiles[actor_index].ttype != TileType::Stairway {
						msglog.tell_player(format!("You can't go {} without a ladder.", dir).as_str());
						continue;
					}
					// CASE 3: Attempt to retrieve a Portal (aka ladder) from the list for this Position
					let possible = model.get_exit(actor_body.ref_posn);
					if let Some(portal) = possible {
						new_location = portal;
					} else {
						msglog.tell_player("Couldn't find a ladder to traverse (possible bug?)");
						continue;
					}
					// CASE 4: The actor is trying to climb higher than the ladder allows
					if dir == Direction::UP && (actor_body.ref_posn.z > new_location.z) {
						msglog.tell_player("You're already at the top of the ladder.");
						continue;
					}
					// CASE 5: The actor is trying to climb lower than the ladder allows
					if dir == Direction::DOWN && (actor_body.ref_posn.z < new_location.z) {
						msglog.tell_player("You're already at the bottom of the ladder.");
						continue;
					}
				}
				let _locn_index = model.levels[new_location.z as usize].to_index(new_location.x, new_location.y);
				// Get a picture of where the actor wants to move to so we can check it for collisions
				let target_extent = actor_body.project_to(new_location);
				//debug!("* target_extent: {:?}", target_extent);
				if let Some(mut blocked_tiles) = model.get_obstructions_at(target_extent, Some(actor_enty)) {
					blocked_tiles.retain(|x| x.1 != Obstructor::Actor(actor_enty));
					// We have a list of positions that are definitely blocked, but we don't know why
					// Get the first one off the list, find out why it's blocked, and report it
					//debug!("blocked tiles: {:?}, {:?}", dir, blocked_tiles);
					let reply_msg = match blocked_tiles[0].1 {
						Obstructor::Actor(enty) => {
							// build an entity message
							let actor = e_query.get(enty).expect("Obstructor actor should be listed in e_query");
							format!("a {}", actor.1.name)
						}
						Obstructor::Object(ttype) => {
							// build a tile message
							format!("a {}", ttype)
						}
					};
					msglog.tell_player(format!("The way {} is blocked by {}", dir, reply_msg).as_str());
					return;
				}
				// -> POINT OF NO RETURN
				// Nothing's in the way, so go ahead and update the actor's position
				//let old_posns = actor_body.extent;
				model.remove_contents(&actor_body.posns(), actor_enty);
				actor_body.move_to(new_location);
				model.add_contents(&actor_body.posns(), 0, actor_enty);
				// If the actor has a Viewshed, flag it as dirty to be updated
				if let Some(mut viewshed) = actor_viewshed {
					viewshed.dirty = true;
				}
				// If the entity changed rooms, update their description to reflect that
				if let Some(new_name) = model.layout.get_room_name(new_location) {
					if new_name != actor_desc.locn {
						actor_desc.locn = format!("{}: {}", new_name, actor_body.ref_posn);
					}
				}
				// If it was the player specifically moving around, we need to do a few more things
				if is_player_action {
					*p_posn_res = new_location; // Update the system-wide resource containing the player's location
					// Is there anything on the ground at the new location?
					// If so, tell the player about it, but don't mention the player entity itself
					let mut contents_list = model.get_contents_at(new_location);
					// "What the heck even is that crazy if-let-Some unwrap statement?"
					// It does the following:
					// 1. creates an iterator from contents_list
					// 2. looks for the position of a specified element to return as a usize
					// 3. the closure obtains the entity using the given entityId,
					// 4. > unwraps it to obtain the entity's components,
					// 5. > and checks to see if it successfully unwrapped a Player component (the '.4.is_some()' field below)
					// 6. > and if so, return the index of that element from the position() function to the index variable
					// 7. which then uses the known-good index variable as an argument to remove the player from the list
					if let Some(index) = contents_list.iter().position(|x| e_query.get(*x).expect("entry of contents_list should be in e_query").4.is_some()) {
						contents_list.remove(index);
					}
					if !contents_list.is_empty() {
						let message = if contents_list.len() <= 3 {
							let mut message_text = "There's a ".to_string();
							loop {
								if let Ok(enty) = e_query.get(contents_list.pop().expect("contents_list should have popped a Some(n)")) {
									if enty.4.is_none() {
										message_text.push_str(&enty.1.name);
									}
								}
								if contents_list.is_empty() { break; }
								else { message_text.push_str(", and a "); }
							}
							message_text.push_str(" here.");
							message_text
						} else {
							"There's some stuff here on the ground.".to_string()
						};
						msglog.tell_player(&message);
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
	                     mut door_query:  Query<(Entity, &mut Body, &Description, &mut Openable, Option<&mut Opaque>, Option<&Obstructive>)>,
	                     mut e_query:     Query<(Entity, &Body, &Description, Option<&Player>, Option<&mut Viewshed>), Without<Openable>>,
) {
	// Bail out if no events or wrong type
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
		let econtext = event.context.as_ref().expect("event.context should be Some(n)");
		// If they can see it, add it to the list of doors they can choose
		let (_enty, _body, a_desc, a_player, a_viewshed) = e_query.get_mut(econtext.subject).expect("actor should be listed in e_query");
		let is_player_action = a_player.is_some();
		let mut message: String = "".to_string();
		match atype {
			ActionType::OpenItem => {
				//debug!("Trying to open a door"); // DEBUG: announce opening a door
				let mut door_name = "".to_string();
				for (d_enty, mut d_body, d_desc, mut d_open, d_opaque, _obstruct) in door_query.iter_mut() {
					if d_enty == econtext.object {
						d_open.is_open = true;
						let ref_posn = d_body.ref_posn; // Get the map posn of the openable
						d_body.set_glyph_at(ref_posn, &d_open.open_glyph); // Change the openable's glyph to the open state
						door_name = d_desc.name.clone();
						if let Some(mut opaque) = d_opaque {
							opaque.opaque = false;
						}
						commands.entity(d_enty).remove::<Obstructive>(); // Things that are open are not obstructive
					}
				}
				if is_player_action {
					message = format!("You open the {}.", door_name);
				} else {
					message = format!("The {} opens a {}.", a_desc.name.clone(), door_name);
				}
				if let Some(mut view) = a_viewshed { view.dirty = true; } // Force a view update ASAP
			}
			ActionType::CloseItem => {
				//debug!("Trying to close a door"); // DEBUG: announce closing door
				let mut door_name = "".to_string();
				for (d_enty, mut d_body, d_desc, mut d_open, d_opaque, _obstruct) in door_query.iter_mut() {
					if d_enty == econtext.object {
						d_open.is_open = false;
						let ref_posn = d_body.ref_posn;
						d_body.set_glyph_at(ref_posn, &d_open.closed_glyph); // Set the openable's glyph to the closed state
						door_name = d_desc.name.clone();
						if let Some(mut opaque) = d_opaque {
							opaque.opaque = true; // Closed things cannot be seen through
						}
						commands.entity(d_enty).insert(Obstructive {}); // Closed things cannot be moved through
					}
				}
				if is_player_action {
					message = format!("You close the {}.", door_name);
				} else {
					message = format!("The {} closes a {}.", a_desc.name.clone(), door_name);
				}
				if let Some(mut view) = a_viewshed { view.dirty = true; }
			}
			_ => { }
		}
		if !message.is_empty() {
			msglog.tell_player(&message);
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
		let econtext = event.context.as_ref().expect("event.context should be Some(n)");
		if econtext.is_blank() { continue; }
		let mut device = d_query.get_mut(econtext.object).expect("econtext.object should be in d_query");
		if !device.2.pw_switch { // If it's not powered on, assume that function first
			device.2.power_toggle();
		}
		// TODO: there's definitely going to be more stuff to implement here depending on the actual Device
	}
}
/// Handles entities that can see physical light
pub fn visibility_system(mut model:  ResMut<WorldModel>,
	                       mut seers:  Query<(&mut Viewshed, &Body, Option<&Player>, Option<&mut Memory>), Changed<Viewshed>>,
	                       //observable: Query<(Entity, &Body)>,
) {
	for (mut s_viewshed, s_body, player, s_memory) in &mut seers {
		if s_viewshed.dirty {
			assert!(s_body.ref_posn.z != -1, "! ERROR: Encountered negative z-level index!");
			let map = &mut model.levels[s_body.ref_posn.z as usize];
			s_viewshed.visible_points.clear();
			// An interesting thought: should an Entity be able to 'see' from every part of its body?
			// Right now it is calculated just from the Entity's reference point, the 'head'
			s_viewshed.visible_points = field_of_view(posn_to_point(&s_body.ref_posn), s_viewshed.range, map);
			s_viewshed.visible_points.retain(|p| p.x >= 0 && p.x < map.width as i32
				                             && p.y >= 0 && p.y < map.height as i32
			);
			if let Some(_player) = player { // if this is the player...
				for s_posn in &s_viewshed.visible_points { // For all the player's visible tiles...
					// ... set the corresponding tile in the map.revealed_tiles to TRUE
					let map_index = map.to_index(s_posn.x, s_posn.y);
					map.revealed_tiles[map_index] = true;
				}
			}
			if let Some(mut recall) = s_memory { // If the seer entity has a memory...
				let mut observations = Vec::new();
				for v_posn in &s_viewshed.visible_points { // Iterate on all points they can see:
					let observed_posn = Position::new(v_posn.x, v_posn.y, s_body.ref_posn.z);
					let observation = model.get_contents_at(observed_posn); // Get the list of observed entities
					let some_observed_entys = if !observation.is_empty() {
						Some(observation)
					} else {
						None
					};
					observations.push((observed_posn, some_observed_entys));
				}
				recall.update(observations);
			}
			s_viewshed.dirty = false;
		}
	}
}

// ###: SINGLETON SYSTEMS
/// Adds a new player entity to a new game world
pub fn new_player_spawn(mut commands: Commands,
	                      spawnpoint:   Res<Position>,
	                      mut model:    ResMut<WorldModel>,
	                      mut p_query:  Query<(Entity, &Player)>,
	                      mut msglog:   ResMut<MessageLog>,
	                      mut global_rng: ResMut<GlobalRng>,
) {
	if !p_query.is_empty() {
		info!("* Existing player found, treating as a loaded game"); // DEBUG: announce possible game load
		let player = p_query.get_single_mut().expect("A loaded game should have a valid player object already");
		commands.entity(player.0).insert(Viewshed::new(8));
		return;
	}
	// DEBUG: testing multitile entities
	// - remove the 'extend()' call from the Body component
	//let extra_posns = vec![
	//	*spawnpoint + (1, 0, 0),
	//	*spawnpoint + (0, 1, 0),
	//	*spawnpoint + (-1, 0, 0),
	//	*spawnpoint + (0, -1, 0),
	//];
	// DEBUG: end testing code
	let player = commands.spawn((
		Player { },
		ActionSet::new(),
		Description::new().name("Pleyeur").desc("Still your old self."),
		*spawnpoint,
		Body::small(*spawnpoint, ScreenCell::new().glyph("@").fg(2).bg(0)),
		Viewshed::new(8),
		Mobile::default(),
		Obstructive::default(),
		Container::default(),
		Memory::new(),
	)).id();
	model.add_contents(&vec![*spawnpoint], 0, player);
	//debug!("* new_player_spawn spawned @{spawnpoint:?}"); // DEBUG: print spawn location of new player
	let planq = commands.spawn((
		Planq::new(),
		Description::new().name("PLANQ").desc("It's your PLANQ."),
		Body::small(*spawnpoint, ScreenCell::new().glyph("¶").fg(3).bg(0)),
		ActionSet::new(),
		Portable::new(player),
		Device::new(-1),
		RngComponent::from(&mut global_rng),
	)).id();
	debug!("* new planq spawned into player inventory: {:?}", planq); // DEBUG: announce creation of player's planq
	commands.spawn(DataSampleTimer::new().source("player_location"));
	commands.spawn(DataSampleTimer::new().source("current_time"));
	commands.spawn(DataSampleTimer::new().source("planq_battery"));
	commands.spawn(DataSampleTimer::new().source("planq_mode"));
	msglog.tell_player("[[fg:green]]WELCOME[[end]] TO [[fg:blue,mod:+italic]]SPACEGAME[[end]]");
}
/// Spawns a new LMR at the specified Position, using default values
pub fn new_lmr_spawn(mut commands:  Commands,
	                   mut msglog:    ResMut<MessageLog>,
) {
	let lmr_spawnpoint = (12, 12, 0).into();
	commands.spawn((
		LMR         { },
		ActionSet::new(),
		Description::new().name("LMR").desc("The Light Maintenance Robot is awaiting instructions."),
		lmr_spawnpoint, // TODO: remove magic numbers
		Body::small(lmr_spawnpoint, ScreenCell::new().glyph("l").fg(14).bg(0)),
		Viewshed::new(5),
		Mobile::default(),
		Obstructive::default(),
		Container::default(),
		Opaque::new(true),
	));
	msglog.add(format!("LMR spawned at {}, {}, {}", 12, 12, 0).as_str(), "debug", 1, 1);
}
/// Adds a demo NPC to the game world
pub fn test_npc_spawn(mut commands: Commands,
	                    mut rng:      ResMut<GlobalRng>,
	                    e_query:      Query<(Entity, &Position)>, // ERROR: replace Position cmp. with Body!
) {
	let spawnpoint = Position::new(rng.i32(1..30), rng.i32(1..30), 0);
	// Check the spawnpoint for collisions
	loop {
		let mut found_open_tile = true;
		for (_enty, posn) in e_query.iter() { // FIXME: this should probably be a call to Model.is_occupied instead
			if posn == &spawnpoint { found_open_tile = false; }
		}
		if found_open_tile { break; }
	}
	commands.spawn((
		ActionSet::new(),
		Description::new().name("Jenaryk").desc("Behold, a generic virtual cariacature of a man."),
		spawnpoint,
		Viewshed::new(8),
		Mobile::default(),
		Obstructive::default(),
		Container::default(),
	));
	//debug!("* Spawned new npc at {}", spawnpoint); // DEBUG: announce npc creation
}

// ###: UTILITIES
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
