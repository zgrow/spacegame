// app/handler.rs
// generated from orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use crate::app::{AppResult, GameEngine, MainMenuItems};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::components::*;
use crate::components::Direction;
use crate::components::GameEventType::*;
use crate::components::Name;
use ratatui::widgets::*;
use bevy::ecs::event::Events;
use bevy::ecs::entity::*;
use bevy::ecs::query::*;

/// Parses the player inputs coming from ratatui and turns them into game logic
pub fn key_parser(key_event: KeyEvent, eng: &mut GameEngine) -> AppResult<()> {
	// WARN: There is an important caveat to this system:
	// Because it is implemented in crossterm via ratatui, making it into a Bevy system
	// has so far been too difficult to finish, if not outright impossible
	// The game_events object below will monopolize the mutable ref to the game world
	// Therefore, do not try to extract and send info now; defer it to Bevy's event handling
	// *** DEBUG KEY HANDLING
	if (key_event.code == KeyCode::Char('c') || key_event.code == KeyCode::Char('C'))
	&& key_event.modifiers == KeyModifiers::CONTROL {
		// Always allow the program to be closed via Ctrl-C
		//eng.running = false;
		eng.quit();
	}
	// Get a linkage to the game event distribution system
	let game_events: &mut Events<GameEvent> = &mut eng.app.world.get_resource_mut::<Events<GameEvent>>().unwrap();
	// *** MENU CONTROL HANDLING
	if eng.main_menu_is_visible {
		// TODO: find a way to generalize this to allow the same logic for inventory selection
		// use the meta mappings
		match key_event.code {
			// Toggle main menu/pause on Esc or Q
			KeyCode::Esc | KeyCode::Char('Q') => {
				eng.pause_toggle();
				eng.main_menu_toggle();
			}
			// Scroll the menu
			KeyCode::Char('j') | KeyCode::Down => {
				eng.main_menu.next();
			}
			KeyCode::Char('k') | KeyCode::Up => {
				eng.main_menu.prev();
			}
			// Allow deselection
			KeyCode::Left => { eng.main_menu.deselect(); }
			// Confirm selection
			KeyCode::Enter => {
				// note that selected() here produces an index to its internal list, not a value!
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
					eng.pause_toggle();
					eng.main_menu_toggle();
					eng.main_menu.deselect();
					return Ok(())
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
				eng.pause_toggle();
			}
			// Pause and show main menu on `ESC` or `Q`
			KeyCode::Esc | KeyCode::Char('Q') => {
				// Close the item chooser if it's open
				if eng.item_chooser_is_visible {
					eng.item_chooser_toggle();
				} else { // Player must be trying to open the main menu
					eng.main_menu_toggle();
					eng.pause_toggle();
				}
			}
			// Move player
			KeyCode::Char('h') | KeyCode::Left => {
				game_events.send(GameEvent{etype: PlayerMove(Direction::W)});
			}
			KeyCode::Char('l') | KeyCode::Right => {
				game_events.send(GameEvent{etype: PlayerMove(Direction::E)});
			}
			KeyCode::Char('k') | KeyCode::Up => {
				if eng.item_chooser_is_visible {
					eng.item_chooser.prev();
				} else {
					game_events.send(GameEvent{etype: PlayerMove(Direction::N)});
				}
			}
			KeyCode::Char('j') | KeyCode::Down => {
				if eng.item_chooser_is_visible {
					eng.item_chooser.next();
				} else {
					game_events.send(GameEvent{etype: PlayerMove(Direction::S)});
				}
			}
			KeyCode::Char('y') => {game_events.send(GameEvent{etype: PlayerMove(Direction::NW)});}
			KeyCode::Char('u') => {game_events.send(GameEvent{etype: PlayerMove(Direction::NE)});}
			KeyCode::Char('b') => {game_events.send(GameEvent{etype: PlayerMove(Direction::SW)});}
			KeyCode::Char('n') => {game_events.send(GameEvent{etype: PlayerMove(Direction::SE)});}
			KeyCode::Char('>') => {game_events.send(GameEvent{etype: PlayerMove(Direction::DOWN)});}
			KeyCode::Char('<') => {game_events.send(GameEvent{etype: PlayerMove(Direction::UP)});}
			KeyCode::Char('o') => {eprintln!("attempted to OPEN something!");}
			//KeyCode::Char('g') => {game_events.send(GameEvent{etype: ItemPickup(Creature::Player)});}
			KeyCode::Char('g') => {
				// FIXME: if there's something here to pick up, show the item chooser
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
				if item_list.len() > 0 { eng.item_chooser_toggle(); }
			}
			KeyCode::Enter     => {
				// If the item chooser is open, operate on that
				if eng.item_chooser_is_visible {
					let choice = eng.item_chooser.state.selected();
					if choice.is_some() {
						let _choice_val = &eng.item_chooser.list[choice.unwrap_or_default()];
						game_events.send(GameEvent{etype: ItemPickup(Creature::Player)});
						// compare the selection to the list...
						eng.item_chooser_toggle();
						eng.item_chooser.deselect();
						return Ok(())
					}
				}
			}
			KeyCode::Char('s') => { // DEBUG: drops a debugging item
				eng.make_item(crate::item_builders::ItemType::Snack, Position::new(30, 20, 0));
			}
			// Other handlers you could add here.
			_ => {}
		}
	}
	Ok(())
}
/// Handles the higher-order logic for Item selection from a group of Items
pub fn item_pickup_menu(_eng: &GameEngine) {

}

// EOF
