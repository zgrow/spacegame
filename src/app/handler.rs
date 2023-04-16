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
	// WARN: There is an important caveat to this system:
	// Because it is implemented in crossterm via ratatui, making it into a Bevy system
	// has so far been too difficult to finish, if not outright impossible
	// The game_events object below will monopolize the mutable ref to the game world
	// Therefore, do not try to extract and send info from here; defer to Bevy's event handling
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
	let planq = &mut eng.app.world.get_resource_mut::<PlanqSettings>().unwrap();
	let mut new_event = GameEvent::default();
	// *** MENU CONTROL HANDLING
	if eng.main_menu_is_visible
	|| eng.item_chooser_is_visible {
		// use the meta mappings
		match key_event.code {
			// Close open menus/unpause on Esc or Q
			KeyCode::Esc | KeyCode::Char('Q') => {
				eng.main_menu_is_visible = false;
				eng.item_chooser_is_visible = false;
				// Dispatch immediately
				let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
				game_events.send(GameEvent::new(ModeSwitch(EngineMode::Running), None));
				return Ok(())
			}
			// Scroll the menu
			KeyCode::Char('j') | KeyCode::Down => {
				if eng.main_menu_is_visible         { eng.main_menu.next(); }
				else if eng.item_chooser_is_visible { eng.item_chooser.next(); }
			}
			KeyCode::Char('k') | KeyCode::Up => {
				if eng.main_menu_is_visible         { eng.main_menu.prev(); }
				else if eng.item_chooser_is_visible { eng.item_chooser.prev(); }
			}
			// Allow deselection
			KeyCode::Char('h') | KeyCode::Left => {
				if eng.main_menu_is_visible         { eng.main_menu.deselect(); }
				else if eng.item_chooser_is_visible { eng.item_chooser.deselect(); }
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
						// Immediate dispatch due to return requirement
						let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
						game_events.send(GameEvent::new(ModeSwitch(EngineMode::Running), None));
						return Ok(())
					}
				} else if eng.item_chooser_is_visible {
					let choice = eng.item_chooser.state.selected();
					if choice.is_some() {
						let choice_val = &eng.item_chooser.list[choice.unwrap_or_default()];
						let context = Some(GameEventContext{subject: player, object: *choice_val});
						eng.item_chooser_toggle(); // close the chooser
						eng.item_chooser.deselect();
						// Immediate dispatch due to return requirement
						let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
						game_events.send(GameEvent::new(ItemMove, context));
						game_events.send(GameEvent::new(ModeSwitch(EngineMode::Running), None));
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
		// use the literal mappings
		match key_event.code {
			// Pause key
			KeyCode::Char('p') => {
				// Dispatch immediately, do not defer
				let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
				game_events.send(GameEvent::new(PauseToggle, None));
				return Ok(())
			}
			// Pause and show main menu on `ESC` or `Q`
			KeyCode::Esc | KeyCode::Char('Q') => {
				// Close the planq chooser if it's open, cancel any in-progress action
				if planq.action_mode != PlanqActionMode::Default {
					eng.planq_chooser.deselect();
					planq.show_inventory = false; // close the inventory prompt if it's open
					planq.action_mode = PlanqActionMode::Default; // exit Drop or Item request
				} else if eng.item_chooser_is_visible {// Close the item chooser if it's open
					eng.item_chooser.deselect();
					eng.item_chooser_is_visible = false;
					// Dispatch immediately, do not defer
					let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
					game_events.send(GameEvent::new(ModeSwitch(EngineMode::Running), None));
					return Ok(())
				} else {// Player must be trying to open the main menu
					eng.main_menu_is_visible = true;
					// Dispatch immediately, do not defer
					let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
					game_events.send(GameEvent::new(ModeSwitch(EngineMode::Paused), None));
					return Ok(())
				}
			}
			// Move player
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
			KeyCode::Char('o') => {eprintln!("attempted to OPEN something!");}
			KeyCode::Char('g') => { // gets the item on the ground (if only one) or invokes item chooser
				let mut item_list = Vec::new();
				let mut item_query = eng.app.world.query_filtered::<(Entity, &Position, &Name), With<Portable>>();
				let p_posn = eng.app.world.get_resource::<Position>().unwrap();
				// Gather up a list of items located at the player's position
				// TODO: Calculate the height of the list given the qty of items
				// WARN: not sure if this will accomodate a too-long list with scrolling...
				// Filter the list down to only those Items with the same loc as the player
				eng.item_chooser.list.clear();
				for item in item_query.iter(&eng.app.world).enumerate() {
					if *item.1.1 == *p_posn {
						item_list.push(ListItem::new(item.1.2.name.clone()));
						eng.item_chooser.list.push(item.1.0);
					}
				}
				if item_list.len() > 0 {
					if item_list.len() > 1 {
						eng.item_chooser_toggle();
						//let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
						//game_events.send(GameEvent::new(ModeSwitch(EngineMode::Paused), None));
						new_event.etype = ModeSwitch(EngineMode::Paused);
					}
					else { // item_list.len == 1
						let choice_val = &eng.item_chooser.list[0];
						new_event.etype = ItemMove;
						new_event.context = Some(GameEventContext{subject: player, object: *choice_val});
					}
				}
			}
			KeyCode::Char('d') => {new_event.etype = PlanqEvent(InventoryDrop);}
			KeyCode::Char('s') => { // DEBUG: drops a snack for testing
				eng.make_item(crate::item_builders::ItemType::Snack, Position::new(30, 20, 0));
			}
			// PLANQ 'sidebar'/ambient control mode
			KeyCode::Left   => {if planq.show_inventory{eng.planq_chooser.deselect();}}
			KeyCode::Right  => { }
			KeyCode::Up     => {if planq.show_inventory{eng.planq_chooser.prev();}}
			KeyCode::Down   => {if planq.show_inventory{eng.planq_chooser.next();}}
			KeyCode::Enter  => {
				if planq.show_inventory {
					let choice = eng.planq_chooser.state.selected();
					if choice.is_some() {
						let choice_val = &eng.planq_chooser.list[choice.unwrap()];
						eprintln!("drop choice: {choice_val:?}");
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
			//  Other handlers you could add here.
			_ => {}
		}
	}
	// If an events was generated, send it off for processing
	// Get a linkage to the game event distribution system
	let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
	game_events.send(new_event);
	Ok(())
}

// EOF
