// sys.rs
// Defines the various subsystems we'll be running on the GameEngine

// NOTE: see bevy/examples/games/alien_cake_addict.rs for example on handling the Player entity

use crate::components::*;
use crate::map::*;
use crate::components::{Name, Position, Renderable, Player, Mobile};
use crate::sys::GameEvent::PlayerMove;
use crate::app::messagelog::MessageLog;
use ratatui::style::{Modifier, Color};
use bevy::ecs::system::{Commands, Res, Query, ResMut};
use bevy::ecs::event::EventReader;
use bevy::ecs::query::Without;
use bracket_pathfinding::prelude::*;

// STARTUP SYSTEMS (run once)
///Spawns a new player, including their subsystems and default values
pub fn new_player_system(mut commands: Commands,
	                     spawnpoint: Res<Position>,
	                     mut msglog: ResMut<MessageLog>,
) {
	commands.spawn((
		// this is the player's collection of components and their initial values
		Player      { },
		Name        {name: "Pleyeur".to_string()},
		Position    {x: spawnpoint.x, y: spawnpoint.y},
		Renderable  {glyph: "@".to_string(), fg: Color::Green, bg: Color::Black, mods: Modifier::empty()},
		Viewshed    {visible_tiles: Vec::new(), range: 8, dirty: true},
		Mobile      { },
	));
	msglog.add("WELCOME TO SPACEGAME".to_string(), "world".to_string(), 1, 1);
}
///Spawns a new LMR at the specified Position, using default values
pub fn new_lmr_system(mut commands: Commands) {
	commands.spawn((
		Name        {name: "LMR".to_string()},
		Position    {x: 12, y: 12}, // FIXME: magic numbers
		Renderable  {glyph: "l".to_string(), fg: Color::LightBlue, bg: Color::Black, mods: Modifier::empty()},
		Viewshed    {visible_tiles: Vec::new(), range: 5, dirty: true},
		Mobile      { },
	));
}

// CONTINUOUS SYSTEMS (run frequently)
/// Handles entities that can move around the map
pub fn movement_system(mut ereader: EventReader<TuiEvent>,
                       map: Res<Map>,
                       mut player_query: Query<(&mut Position, &Player, &mut Viewshed)>,
                       mut _npc_query: Query<((&Position, &Mobile, Option<&mut Viewshed>), Without<Player>)>
) {
	//typical bevy-based method just goes through the stack of inputs and matches them up
	//would rather have something input-indepdendent here, so that the LMR can be run as well
	for event in ereader.iter() {
		eprintln!("player attempting to move");
		match event.etype {
			PlayerMove(dir) => {
				let (mut p_pos, _player, mut pview) = player_query.single_mut();
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
				if map.is_occupied(target) { // FIXME: does not check entity collision!
					return;
				}
				p_pos.x = target.x;
				p_pos.y = target.y;
				pview.dirty = true;
			}
			//this is where we'd handle an NPCMove action
		}
	}
}
/// Handles entities that can see physical light
pub fn visibility_system(mut map: ResMut<Map>,
                         mut seers: Query<(&mut Viewshed, &Position, Option<&Player>)>
) {
	for (mut viewshed, posn, player) in &mut seers {
		if viewshed.dirty {
			viewshed.visible_tiles.clear();
			viewshed.visible_tiles = field_of_view(posn_to_point(posn), viewshed.range, &*map);
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
// TODO: revealed_system: if the layer were painted with the list of renderables before any of the
// rooms were deformed by way of 'finishing' level generation touches, it would provide a 'prior'
// memory of the room layouts - consider allowing updated memory of certain objects?
/// Converts a spacegame::Position into a bracket_pathfinding::Point
pub fn posn_to_point(input: &Position) -> Point { Point { x: input.x, y: input.y } }

// EOF
