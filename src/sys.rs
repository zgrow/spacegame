// sys.rs
// Defines the various subsystems we'll be running on the GameEngine

// NOTE: see bevy/examples/games/alien_cake_addict.rs for example on handling the Player entity

use crate::components::*;
use crate::components::{Name, Position, Renderable, Player, Mobile};
use crate::sys::GameEvent::PlayerMove;
use ratatui::style::{Modifier, Color};
use bevy::ecs::system::{Commands, Res, Query};
use bevy::ecs::event::EventReader;
use bevy::ecs::query::Without;

// STARTUP SYSTEMS (run once)
pub fn new_player_system(mut commands: Commands, spawnpoint: Res<Position>) {
	commands.spawn((
		// this is the player's collection of components and their initial values
		Player      { },
		Name        {name: "Pleyeur".to_string()},
		Position    {x: spawnpoint.x, y: spawnpoint.y},
		Renderable  {glyph: "@".to_string(), fg: Color::Green, bg: Color::Black, mods: Modifier::empty()},
		Mobile      { },
	));
}

// CONTINUOUS SYSTEMS (run frequently)
/// Handles entities that can move around the map
pub fn movement_system(mut _commands: Commands,
                       mut ereader: EventReader<TuiEvent>,
                       mut player_query: Query<(&mut Position, &Player)>,
                       mut _npc_query: Query<((&Position, &Mobile), Without<Player>)>
) {
	//typical bevy-based method just goes through the stack of inputs and matches them up
	//would rather have something input-indepdendent here, so that the LMR can be run as well
	let (mut p_pos, _player) = player_query.single_mut();
	for event in ereader.iter() {
		eprintln!("player attempting to move");
		match event.etype {
			PlayerMove(dir) => {
				// FIXME: no collision or bounds checking is done!
				match dir {
					Direction::N  => { p_pos.y -= 1 }
					Direction::NW => { p_pos.x -= 1; p_pos.y -= 1 }
					Direction::W  => { p_pos.x -= 1 }
					Direction::SW => { p_pos.x -= 1; p_pos.y += 1 }
					Direction::S  => { p_pos.y += 1 }
					Direction::SE => { p_pos.x += 1; p_pos.y += 1 }
					Direction::E  => { p_pos.x += 1 }
					Direction::NE => { p_pos.x += 1; p_pos.y -= 1 }
				}
			}
			//this is where we'd handle an NPCMove action
		}
	}
}

// EOF
