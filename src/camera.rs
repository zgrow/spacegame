/// engine/camera.rs
/// Provides implementation for the CameraView component, including refresh/update logic

// *** EXTERNAL LIBS
use bevy::prelude::*;
use bevy::ecs::event::EventReader;
use bevy::ecs::query::Without;
use bracket_geometry::prelude::*;

// *** INTERNAL LIBS
use crate::components::*;
use crate::map::*;
use crate::engine::event::*;

/// Represents a 'flattened' view of the Map's layers, with all entities and effects painted in,
/// such that it can be read by the Viewport object when it comes time to render the view
/// Provides an abstraction to the Viewport widget with hooks into Bevy's systems for updates
#[derive(Component, Resource, Clone, Debug, Default, Reflect)]
#[reflect(Component, Resource)]
pub struct CameraView {
	pub map: Vec<Tile>,
	pub width: i32,
	pub height: i32,
	//pub fx: Vec<VisualEffect>,
	pub reticle: Position,
	pub reticle_glyphs: String,
}
impl CameraView {
	pub fn new(new_width: i32, new_height: i32) -> Self {
		Self {
			map: vec![Tile::default(); (new_width * new_height) as usize],
			width: new_width,
			height: new_height,
			//fx: Vec::new(),
			reticle: Position::INVALID,
			reticle_glyphs: "⌟⌞⌝⌜".to_string(), // Corner frame
		}
		// Other options for reticles might include: (not all tested)
		// The reticle glyph order is UL, UR, DL, DR
		//	reticle_glyphs: "JL7F".to_string(), // 1337code fallback, guaranteed to work everywhere lol
		//	reticle_glyphs: "\\//\\".to_string(), // Slashes in the corners
		//	reticle_glyphs: "▗▖▝▘".to_string(), // Four small corner boxes
		//	reticle_glyphs: "▚▞▞▚".to_string(), // Blocky slashes
		//	reticle_glyphs: "◞◟◝◜".to_string(), // Curved corners (might have unicode issues)
		//	reticle_glyphs: "◿◺◹◸".to_string(), // Triangular corners
		//	reticle_glyphs: "⌟⌞⌝⌜".to_string(), // Corner frame
		//	reticle_glyphs: "⌌⌍⌎⌏".to_string(), // Square frame
		//	reticle_glyphs: "|\/".to_string(), // need to impl a 3-point reticle in the logic below
	}
	pub fn set_dims(&mut self, new_width: i32, new_height: i32) {
		// TODO: include a sanity check here that actually examines the dims prior to resize
		// if the resize is required, then probably safest to wipe the whole thing...
		// either way, make sure that the CameraView gets an update before next render call
		self.width = new_width;
		self.height = new_height;
		let new_size = (self.width * self.height) as usize;
		if self.map.len() != new_size {
			self.map = vec![Tile::default(); new_size];
		}
	}
	/*
	pub fn iterate_countdown(&mut self) {
		for effect in self.fx.iter_mut() {
			if effect.countdown > 0 { effect.countdown -= 1; }
		}
	}
	*/
}
/// Provides the update system for Bevy
pub fn camera_update_system(mut camera:   ResMut<CameraView>,
	                          renderables:  Query<(&Position, &Renderable), Without<Player>>,
	                          model:        Res<Model>,
	                          mut q_player: Query<(&Player, &Viewshed, &Position, &Renderable, &Memory)>,
	                          mut _ereader: EventReader<GameEvent>,
) {
	if q_player.get_single_mut().is_err() { return; }
	let player = q_player.get_single_mut().unwrap();
	let world_map = &model.levels[player.2.z as usize];
	// Absolutely positively do not try to do this if the camera or map are empty
	assert!(!camera.map.is_empty(), "camera.map has length 0!");
	assert!(!world_map.tiles.is_empty(), "world_map.tiles has length 0!");
	let cam_width = camera.width as usize;
	let centerpoint = Position::create((cam_width / 2) as i32, camera.height / 2, 0);
	let minima = Position::create(player.2.x - centerpoint.x, player.2.y - centerpoint.y, 0);
	let maxima = Position::create(player.2.x + centerpoint.x, player.2.y + centerpoint.y, 0);
	for (screen_y, target_y) in (minima.y..maxima.y).enumerate() {
		for (screen_x, target_x) in (minima.x..maxima.x).enumerate() {
			// We are iterating on target_x/y AND screen_x/y
			// Update the world_map and buf indices at the same time to avoid confusion
			let buf_index = xy_to_index(screen_x, screen_y, cam_width);
			let map_index = world_map.to_index(target_x, target_y);
			let mut new_tile = Tile::default();
			// Check for an existing tile in the world_map
			// Don't use map_index to perform the bounds check:
			//   it'll map to ANY valid index, too many false positives
			// IF the target_x/y produces a valid world_map coordinate...
			if target_x >= 0 && target_x < world_map.width
			&& target_y >= 0 && target_y < world_map.height
			&& world_map.revealed_tiles[map_index] { // and if the tile's been seen before...
				// ... THEN put together the displayed tile from various input sources:
				// First, obtain the background
				new_tile = world_map.tiles[map_index].clone();
				new_tile.bg = 8; // DEBUG: force an 'illuminated' background color
				if target_x == player.2.x && target_y == player.2.y { // Is this position where the player is standing?
					// If so, draw the player and move on
					new_tile.glyph = player.3.glyph.clone();
					new_tile.fg = player.3.fg;
					//new_tile.bg = player.3.bg; // DEBUG: use bg above
					new_tile.mods = "".to_string();
				} else if player.1.visible_tiles.contains(&Point::new(target_x, target_y)) { // Render all visible things
					if !&renderables.is_empty() {
						for (posn, rendee) in &renderables {
							if (posn.x, posn.y, posn.z) == (target_x, target_y, player.2.z) {
								new_tile.glyph = rendee.glyph.clone();
								new_tile.fg = rendee.fg;
								//new_tile.bg = rendee.bg; // DEBUG: use bg above
								new_tile.mods = "".to_string();
							}
						}
					}
				} else { // Render everything that is 'out of sight', ie the previously-seen tiles and entities
					// Otherwise, just assume this is a real-but-not-visible tile and recolor it
					new_tile.fg = 8;
					new_tile.bg = 0;
					new_tile.mods = "".to_string();
					// If we recall an Entity at this location then use that glyph for display
					if let Some(enty) = player.4.visual.iter().find(|&x| *x.1 == Position::create(target_x, target_y, player.2.z)) {
						if let Ok(display) = renderables.get(*enty.0) {
							new_tile.glyph = display.1.glyph.clone();
						}
					}
				}
			} else {
				// ... ELSE just make it a background tile (ie starfield)
				new_tile.glyph = "░".to_string();
			}
			// Iterate through the list of visual effects and apply any that the player can see
			/* *** This section disabled for now
			if !camera.fx.is_empty() {
				camera.iterate_countdown();
				// NOTE: Have to use an index loop here because Vec::drain_filter is still in Rust nightly
				let mut f_index = 0;
				while f_index < camera.fx.len() {
					if camera.fx[f_index].countdown == 0 { // Only activate effects that are supposed to go off right now
						// Get the centerpoint of the effect
						let centerpoint = camera.fx[f_index].position;
						// For each element of the effect, check if it is visible to the player and draw it if so
						let mut e_index = 0;
						while e_index < camera.fx[f_index].elements.len() {
							let element = camera.fx[f_index].elements[e_index].clone();
							// Get the actual map position of the element
							let (e_x, e_y) = (centerpoint.x + element.1, centerpoint.y + element.2);
							if player.1.visible_tiles.contains(&Point::new(e_x, e_y)) {
								// Get the screen position of the element
								let e_index = xy_to_index(e_x, e_y, cam_width);
								camera.map[e_index].glyph = element.0;
							}
							e_index += 1;
						}
					}
					f_index += 1;
				}
				camera.fx.retain(|x| x.countdown > 0);
			}*/
			if camera.reticle != Position::INVALID {
				// TODO: Add some logic that will detect other entity positions (such as the player!) and choose
				//       a reticle shape that minimizes the number of entities who will be hidden by the points
				// TODO: Add a line-of-sight ruler that can show where the LOS is blocked with line coloration
				let ul_index = xy_to_index(camera.reticle.x as usize - 1, camera.reticle.y as usize - 1, cam_width);
				let ur_index = xy_to_index(camera.reticle.x as usize + 1, camera.reticle.y as usize - 1, cam_width);
				let dl_index = xy_to_index(camera.reticle.x as usize - 1, camera.reticle.y as usize + 1, cam_width);
				let dr_index = xy_to_index(camera.reticle.x as usize + 1, camera.reticle.y as usize + 1, cam_width);
				let ret_chars = camera.reticle_glyphs.clone();
				for (index, corner) in ret_chars.chars().enumerate() {
					match ret_chars.chars().count() {
						3 => { /* TODO: impl logic for 3-point reticles */ }
						4 => {
							match index {
								0 => {camera.map[ul_index].glyph = corner.to_string(); camera.map[ul_index].fg = 11; camera.map[ul_index].bg = 8;}
								1 => {camera.map[ur_index].glyph = corner.to_string(); camera.map[ur_index].fg = 11; camera.map[ur_index].bg = 8;}
								2 => {camera.map[dl_index].glyph = corner.to_string(); camera.map[dl_index].fg = 11; camera.map[dl_index].bg = 8;}
								3 => {camera.map[dr_index].glyph = corner.to_string(); camera.map[dr_index].fg = 11; camera.map[dr_index].bg = 8;}
								_ => { }
							}
						}
						_ => { }
					}
				}
			}
			camera.map[buf_index] = new_tile;
		}
	}
}
/* Disabled pending implementation finish
/// Represents a single visual effect to be applied to the CameraView, ie a targeting reticle or explosion effect
#[derive(Component, Resource, Clone, Debug, Default, Reflect)]
pub struct VisualEffect {
	/// How long until the visual effect will be triggered
	pub countdown: i32,
	/// The map position that the effect was triggered at
	pub position: Position,
	/// The list of visual elements that need to be drawn
	/// Each triplet is a single char, plus x and y *offsets*
	pub elements: Vec<(String, i32, i32)>
}
impl VisualEffect { // TODO: add builders to this instead of lumping it into one fxn
	fn new(time: i32, locn: Position, fx: Vec<(String, i32, i32)>) -> Self {
		VisualEffect {
			countdown: time,
			position: locn,
			elements: fx,
		}
	}
}
*/

// EOF
