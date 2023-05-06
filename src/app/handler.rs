// app/handler.rs
// generated from orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::*;
use bevy::ecs::event::Events;
use bevy::ecs::entity::*;
use bevy::ecs::query::*;

use crate::app::{AppResult, GameEngine, MainMenuItems};
use crate::app::planq::*;
use crate::app::planq::PlanqActionMode::*;
use crate::components::*;
use crate::components::Direction;
use crate::components::GameEventType::*;
use crate::components::PlanqEventType::*;
use crate::components::Name;

/// Parses the player inputs coming from ratatui and turns them into game logic
pub fn key_parser(key_event: KeyEvent, eng: &mut GameEngine) -> AppResult<()> {
	// WARN: STOP TRYING TO USE BEVY QUERIES IN THIS METHOD, it WILL cause ownership issues!
	// Either you meant to send a control command to the Planq,
	//  you forgot to defer/delegate the data query to a Bevy system,
	//  or if you're trying to control the GameEngine, consider abstracting up to the GameEngine
	/* Because it is implemented in crossterm via ratatui, making it into a Bevy system
	 * has so far been too difficult to finish, if not outright impossible
	 * The game_events object below will monopolize the mutable ref to the game world
	 * Therefore, do not try to extract and send info from here; defer to Bevy's event handling
	 */
	// *** DEBUG KEY HANDLING
	if (key_event.code == KeyCode::Char('c') || key_event.code == KeyCode::Char('C'))
	&& key_event.modifiers == KeyModifiers::CONTROL {
		// Always allow the program to be closed via Ctrl-C
		//eng.running = false;
		eng.quit();
	}
	// Extract entity ids for the player and the player's planq
	let mut player_query = eng.app.world.query_filtered::<Entity, With<Player>>();
	let player = player_query.get_single(&eng.app.world).unwrap();
	// *** MENU CONTROL HANDLING
	if eng.main_menu_is_visible
	|| eng.item_chooser_is_visible
	|| eng.target_chooser_is_visible
	{
		// use the meta mappings
		match key_event.code {
			// Only handle these keys if the game's actually in-progress
			// Close open menus/unpause on Esc or Q
			KeyCode::Esc | KeyCode::Char('Q') => {
				// Only handle this if the game's actually running
				if eng.standby { return Ok(()); }
				eng.main_menu_is_visible = false;
				eng.item_chooser_is_visible = false;
				eng.target_chooser_is_visible = false;
				// Dispatch immediately
				eng.pause_game(false);
				return Ok(())
			}
			// Scroll the menu
			KeyCode::Char('j') | KeyCode::Down => {
				if eng.main_menu_is_visible         { eng.main_menu.next(); }
				else if eng.item_chooser_is_visible { eng.item_chooser.next(); }
				else if eng.target_chooser_is_visible { eng.target_chooser.next(); }
			}
			KeyCode::Char('k') | KeyCode::Up => {
				if eng.main_menu_is_visible         { eng.main_menu.prev(); }
				else if eng.item_chooser_is_visible { eng.item_chooser.prev(); }
				else if eng.target_chooser_is_visible { eng.target_chooser.prev(); }
			}
			// Allow deselection
			KeyCode::Char('h') | KeyCode::Left => {
				if eng.main_menu_is_visible         { eng.main_menu.deselect(); }
				else if eng.item_chooser_is_visible { eng.item_chooser.deselect(); }
				else if eng.target_chooser_is_visible { eng.target_chooser.deselect(); }
			}
			// Confirm selection
			KeyCode::Enter => {
				// note that selected() here produces an index to its internal list, not a value!
				if eng.main_menu_is_visible {
					let choice = eng.main_menu.state.selected();
					if choice.is_some() {
						let choice_val = &eng.main_menu.list[choice.unwrap_or_default()]; // the list value itself
						match choice_val {
							MainMenuItems::NEWGAME => {
								eprintln!("NEWGAME called"); // DEBUG:
								if eng.standby { eng.standby = false; return Ok(()); }
							}
							MainMenuItems::LOADGAME => {
								eng.load_game();
							}
							MainMenuItems::SAVEGAME => {
								eng.save_game();
							}
							MainMenuItems::QUIT => {
								eprintln!("QUIT called"); // DEBUG:
								eng.quit();
								return Ok(())
							}
							MainMenuItems::NULL => { }
						}
						// Then clear off the screen and return to the game
						eng.main_menu_toggle();
						eng.main_menu.deselect();
						eng.pause_game(false);
						return Ok(())
					}
				}
				else if eng.item_chooser_is_visible {
					let choice = eng.item_chooser.state.selected();
					if choice.is_some() {
						let choice_val = &eng.item_chooser.list[choice.unwrap_or_default()];
						if *choice_val == Entity::PLACEHOLDER { return Ok(()); }
						let context = Some(GameEventContext{subject: player, object: *choice_val});
						eng.hide_item_chooser(); // close the chooser
						eng.item_chooser.deselect();
						// Immediate dispatch due to return requirement
						let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
						game_events.send(GameEvent::new(eng.player_action, context));
						eng.pause_game(false);
						return Ok(())
					}
				}
				else if eng.target_chooser_is_visible {
					let choice = eng.target_chooser.state.selected();
					if choice.is_some() {
						let choice_val = &eng.target_chooser.list[choice.unwrap_or_default()];
						if *choice_val == Entity::PLACEHOLDER { return Ok(()); }
						let context = Some(GameEventContext{subject: player, object: *choice_val});
						eng.hide_target_chooser();
						eng.target_chooser.deselect();
						// Immediate dispatch due to return requirement
						let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
						game_events.send(GameEvent::new(eng.player_action, context));
						eng.pause_game(false);
						return Ok(())
					}
				}
			}
			// Else, do nothing
			_ => {}
		}
	}
	// *** GAME CONTROL HANDLING
	else { // this is the 'default' game interaction mode
		// If the game is paused, don't accept any other key inputs
		if eng.mode == EngineMode::Paused
		&& key_event.code != KeyCode::Char('p')
		&& key_event.code != KeyCode::Char('Q')
		&& key_event.code != KeyCode::Esc
		{ return Ok(()) }
		let mut new_event = GameEvent::default(); // etype will be GameEventType::NullEvent
		let planq = &mut eng.app.world.get_resource_mut::<PlanqSettings>().unwrap();
		match key_event.code {
			// Meta actions
			KeyCode::Char('p') => { // Pause key toggle
				// Dispatch immediately, do not defer
				eng.pause_toggle();
				return Ok(())
			}
			KeyCode::Esc | KeyCode::Char('Q') => { // Pause and show main menu on `ESC` or `Q`
				// Close the planq chooser if it's open, cancel any in-progress action
				if planq.action_mode != PlanqActionMode::Default {
					eng.planq_chooser.deselect();
					planq.show_inventory = false; // close the inventory prompt if it's open
					planq.action_mode = PlanqActionMode::Default; // exit Drop or Item request
/*				} else if eng.item_chooser_is_visible {// Close the item chooser if it's open
					eng.item_chooser.deselect();
					eng.item_chooser_is_visible = false;
					eng.pause_game(false);
					return Ok(())
				} else if eng.target_chooser_is_visible {// Close the target chooser if it's open
					eng.target_chooser.deselect();
					eng.item_chooser_is_visible = false;
					eng.pause_game(false); */
				} else {// Player must be trying to open the main menu
					eng.main_menu_is_visible = true;
					// Dispatch immediately, do not defer
					eng.pause_game(true);
					return Ok(())
				}
			}
			// Simple actions, no context required
			KeyCode::Char('h') => {new_event.etype = PlayerMove(Direction::W);}
			KeyCode::Char('l') => {new_event.etype = PlayerMove(Direction::E);}
			KeyCode::Char('k') => {new_event.etype = PlayerMove(Direction::N);}
			KeyCode::Char('j') => {new_event.etype = PlayerMove(Direction::S);}
			KeyCode::Char('y') => {new_event.etype = PlayerMove(Direction::NW);}
			KeyCode::Char('u') => {new_event.etype = PlayerMove(Direction::NE);}
			KeyCode::Char('b') => {new_event.etype = PlayerMove(Direction::SW);}
			KeyCode::Char('n') => {new_event.etype = PlayerMove(Direction::SE);}
			KeyCode::Char('>') => {new_event.etype = PlayerMove(Direction::DOWN);}
			KeyCode::Char('<') => {new_event.etype = PlayerMove(Direction::UP);}
			KeyCode::Char('i') => {new_event.etype = PlanqEvent(InventoryUse);}
			KeyCode::Char('d') => {new_event.etype = PlanqEvent(InventoryDrop);}
			// Compound actions, context required: may require secondary inputs from player
			KeyCode::Char('o') => { // OPEN an Openable nearby
				let mut open_names = Vec::new();
				let mut open_query = eng.app.world.query::<(Entity, &Position, &Name, &Openable)>();
				let p_posn = *eng.app.world.get_resource::<Position>().unwrap();
				//eprintln!("attempted to OPEN at posn {p_posn:?}"); // DEBUG:
				eng.target_chooser.list.clear();
				for target in open_query.iter(&eng.app.world) {
					if target.1.in_range_of(p_posn, 1)
					&& !target.3.is_open
					{
						//eprintln!("Found a door to open: {}", target.2.name.clone()); // DEBUG:
						open_names.push(ListItem::new(target.2.name.clone())); // display list
						eng.target_chooser.list.push(target.0);
					}
				}
				if open_names.len() > 0 {
					if open_names.len() == 1 {
						let choice_val = eng.target_chooser.list[0];
						new_event.etype = ActorOpen;
						new_event.context = Some(GameEventContext { subject: player, object: choice_val });
						//eprintln!("new event: {}, {choice_val:?}", new_event.etype); // DEBUG:
					} else {
						eng.pause_game(true);
						eng.player_action = ActorOpen;
						eng.show_target_chooser();
					}
				}
			}
			KeyCode::Char('c') => { // CLOSE an Openable nearby
				//eprintln!("attempted to CLOSE something!"); // DEBUG:
				let mut close_names = Vec::new();
				let mut close_query = eng.app.world.query::<(Entity, &Position, &Name, &Openable)>();
				let p_posn = *eng.app.world.get_resource::<Position>().unwrap();
				eng.target_chooser.list.clear();
				for target in close_query.iter(&eng.app.world) {
					if target.1.in_range_of(p_posn, 1)
					&& target.3.is_open
					{
						close_names.push(ListItem::new(target.2.name.clone()));
						eng.target_chooser.list.push(target.0);
					}
				}
				if close_names.len() > 0 {
					if close_names.len() == 1 {
						let choice_val = eng.target_chooser.list[0];
						new_event.etype = ActorClose;
						new_event.context = Some(GameEventContext { subject: player, object: choice_val });
					} else {
						eng.pause_game(true);
						eng.player_action = ActorClose;
						eng.show_target_chooser();
					}
				}
			}
			KeyCode::Char('g') => { // GET a Portable item from the ground at player's feet
				let mut item_names = Vec::new();
				let mut item_query = eng.app.world.query_filtered::<(Entity, &Position, &Name), With<Portable>>();
				let p_posn = *eng.app.world.get_resource::<Position>().unwrap();
				// Filter the list by entities in range (ie only the ones at the player's feet)
				eng.item_chooser.list.clear();
				for item in item_query.iter(&eng.app.world) {
					if *item.1 == p_posn {
						item_names.push(ListItem::new(item.2.name.clone()));
						eng.item_chooser.list.push(item.0);
					}
				}
				if item_names.len() > 0 { // Were any items found?
					if item_names.len() == 1 { // YES: exactly 1, so use it in the action
						let choice_val = eng.item_chooser.list[0];
						new_event.etype = ItemMove;
						new_event.context = Some(GameEventContext{ subject: player, object: choice_val });
						//eprintln!("attempted to pick up {choice_val:?}"); // DEBUG:
					} else { // YES: 2+, so ask the player to clarify
						eng.pause_game(true);
						eng.player_action = ItemMove;
						eng.show_item_chooser();
					}
				}
			}
			// PLANQ 'sidebar'/ambient controls
			KeyCode::Left   => {if planq.show_inventory{eng.planq_chooser.deselect();}}
			KeyCode::Right  => { /* does nothing in this context */ }
			KeyCode::Up     => {if planq.show_inventory{eng.planq_chooser.prev();}}
			KeyCode::Down   => {if planq.show_inventory{eng.planq_chooser.next();}}
			KeyCode::Enter  => {
				if planq.show_inventory {
					let choice = eng.planq_chooser.state.selected();
					if choice.is_some() {
						let choice_val = &eng.planq_chooser.list[choice.unwrap()];
						//eprintln!("drop choice: {choice_val:?}"); // DEBUG:
						new_event.context = Some(GameEventContext{subject: player, object: *choice_val});
					}
					match planq.action_mode {
						Default =>  { /* do nothing, there shouldn't even be an open menu */ }
						DropItem => {new_event.etype = ItemDrop;}
						UseItem =>  {new_event.etype = ItemUse;}
					}
					planq.show_inventory = false;
					eng.planq_chooser.deselect();
				}
			}
			// Debug keys and other tools
			KeyCode::Char('s') => { // DEBUG: drops a snack for testing
				eng.make_item(crate::item_builders::ItemType::Snack, Position::new(30, 20, 0));
			}
			//  Other handlers you could add here.
			_ => {}
		}
		// If an event was generated, send it off for processing
		if new_event.etype != GameEventType::NullEvent {
			// Get a linkage to the game event distribution system
			let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
			game_events.send(new_event);
		}
	}
	Ok(())
}

// EOF
