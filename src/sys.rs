// sys.rs
// Defines the various subsystems we'll be running on the GameEngine

use crate::components::{Name, Position, Renderable};
use ratatui::style::{Modifier, Color};
use bevy::ecs::system::{Commands, Res};

pub fn new_player_system(mut commands: Commands, spawnpoint: Res<Position>) {
	commands.spawn((
		// this is the player's collection of components and their initial values
		Name        {name: "Pleyeur".to_string()},
		Position    {x: spawnpoint.x, y: spawnpoint.y},
		Renderable  {glyph: "@".to_string(), fg: Color::Green, bg: Color::Black, mods: Modifier::empty()},
	));
}

// EOF
