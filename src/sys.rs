// sys.rs
// Defines the various subsystems we'll be running on the GameEngine

// NOTE: see bevy/examples/games/alien_cake_addict.rs for example on handling the Player entity

use crate::components::*;
use crate::map::*;
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
pub fn movement_system(mut ereader: EventReader<TuiEvent>,
                       map: Res<Map>,
                       mut player_query: Query<(&mut Position, &Player)>,
                       mut _npc_query: Query<((&Position, &Mobile), Without<Player>)>
) {
	//typical bevy-based method just goes through the stack of inputs and matches them up
	//would rather have something input-indepdendent here, so that the LMR can be run as well
	for event in ereader.iter() {
		eprintln!("player attempting to move");
		match event.etype {
			PlayerMove(dir) => {
				let (mut p_pos, _player) = player_query.single_mut();
				let mut xdiff = 0;
				let mut ydiff = 0;
				match dir {
					Direction::N  =>             { ydiff -= 1 }
					Direction::NW => { xdiff -= 1; ydiff -= 1 }
					Direction::W  => { xdiff -= 1 }
					Direction::SW => { xdiff -= 1; ydiff += 1 }
					Direction::S  =>             { ydiff += 1 }
					Direction::SE => { xdiff += 1; ydiff += 1 }
					Direction::E  => { xdiff += 1 }
					Direction::NE => { xdiff += 1; ydiff -= 1 }
				}
				let target = Position{x: p_pos.x + xdiff, y: p_pos.y + ydiff};
				if map.is_occupied(target) {
					return;
				}
				p_pos.x = target.x;
				p_pos.y = target.y;
			}
			//this is where we'd handle an NPCMove action
		}
	}
}

// EOF
