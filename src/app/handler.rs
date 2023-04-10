// app/handler.rs
// generated from orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use crate::app::{AppResult, GameEngine, MainMenuItems};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::components::*;
use crate::components::Direction;
use crate::components::GameEventType::*;
use bevy::ecs::event::Events;

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
	if eng.show_main_menu {
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
	// *** GAME CONTROL HANDLING
	} else { // this should be the 'default' game interaction mode
		// use the literal mappings
		match key_event.code {
			// Pause key
			KeyCode::Char('p') => {
				eng.pause_toggle();
			}
			// Pause and show main menu on `ESC` or `Q`
			KeyCode::Esc | KeyCode::Char('Q') => {
				eng.pause_toggle();
				eng.main_menu_toggle();
			}
			// Move player
			KeyCode::Char('h') | KeyCode::Left => {
				game_events.send(GameEvent{etype: PlayerMove(Direction::W)});
			}
			KeyCode::Char('l') | KeyCode::Right => {
				game_events.send(GameEvent{etype: PlayerMove(Direction::E)});
			}
			KeyCode::Char('j') | KeyCode::Down => {
				game_events.send(GameEvent{etype: PlayerMove(Direction::S)});
			}
			KeyCode::Char('k') | KeyCode::Up => {
				game_events.send(GameEvent{etype: PlayerMove(Direction::N)});
			}
			KeyCode::Char('y') => {game_events.send(GameEvent{etype: PlayerMove(Direction::NW)});}
			KeyCode::Char('u') => {game_events.send(GameEvent{etype: PlayerMove(Direction::NE)});}
			KeyCode::Char('b') => {game_events.send(GameEvent{etype: PlayerMove(Direction::SW)});}
			KeyCode::Char('n') => {game_events.send(GameEvent{etype: PlayerMove(Direction::SE)});}
			KeyCode::Char('>') => {game_events.send(GameEvent{etype: PlayerMove(Direction::DOWN)});}
			KeyCode::Char('<') => {game_events.send(GameEvent{etype: PlayerMove(Direction::UP)});}
			KeyCode::Char('o') => {eprintln!("attempted to OPEN something!");}
			KeyCode::Char('g') => {game_events.send(GameEvent{etype: ItemPickup(Creature::Player)});}
			KeyCode::Char('s') => {eng.make_item(crate::item_builders::ItemType::Thing, Position::new(10, 10, 0));}
			// Other handlers you could add here.
			_ => {}
		}
	}
	Ok(())
}

// EOF
