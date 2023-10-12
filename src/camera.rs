// engine/camera.rs
// Provides implementation for the CameraView component, including refresh/update logic

#![allow(clippy::type_complexity)]

// *** EXTERNAL LIBS
use bevy::prelude::*;
use bevy::ecs::query::Without;
use bracket_geometry::prelude::*;
use ratatui::style::Color;
use ratatui::buffer::Cell;
use ratatui::style::Modifier;

// *** INTERNAL LIBS
use crate::components::*;
use crate::map::*;
use crate::artisan::furniture::Facade;


/// Represents a 'flattened' view of the Map's layers, with all entities and effects painted in,
/// such that it can be read by the Viewport object when it comes time to render the view
/// Provides an abstraction to the Viewport widget with hooks into Bevy's systems for updates
#[derive(Component, Resource, Clone, Debug, Default, Reflect)]
#[reflect(Component, Resource)]
pub struct CameraView {
	pub output: Vec<ScreenCell>,
	pub width: i32,
	pub height: i32,
	pub reticle: Position,
	pub reticle_glyphs: String,
	pub terrain: Vec<ScreenCell>, // The map of all the 'base' tiles, provides the 'fallback' minimum visual for rendering
	pub scenery: Vec<ScreenCell>, // The map of all the scenery and furniture objects
	pub actors: Vec<ScreenCell>, // The map of all the Actor glyphs
	pub blinken: Vec<ScreenCell>, // The map of all the 'scenery' effects: glowing screens and other cycling animations
	pub vfx: Vec<ScreenCell>, // The map of the special effects feedback visuals - short-term and incidental
}
impl CameraView {
	pub fn new(new_width: i32, new_height: i32) -> Self {
		Self {
			output: vec![ScreenCell::default(); (new_width * new_height) as usize],
			width: new_width,
			height: new_height,
			reticle: Position::INVALID,
			reticle_glyphs: "⌟⌞⌝⌜".to_string(), // Corner frame
			terrain: vec![ScreenCell::default(); (new_width * new_height) as usize],
			scenery: vec![ScreenCell::default(); (new_width * new_height) as usize],
			actors: vec![ScreenCell::default(); (new_width * new_height) as usize],
			blinken: vec![ScreenCell::default(); (new_width * new_height) as usize],
			vfx: vec![ScreenCell::default(); (new_width * new_height) as usize],
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
		if self.output.len() != new_size {
			self.output = vec![ScreenCell::default(); new_size];
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
/// Compatibility type for better integration with ratatui; converts directly to a ratatui::Buffer::Cell
#[derive(Component, Resource, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Resource)]
pub struct ScreenCell {
	pub glyph: String,
	pub fg: u8,
	pub bg: u8,
	pub modifier: u16,
	// The Cell::underline_color and Cell::skip fields are not needed
}
impl From<ScreenCell> for Cell {
	fn from(input: ScreenCell) -> Self {
		Cell {
			symbol: input.glyph.clone(),
			fg: Color::Indexed(input.fg),
			bg: Color::Indexed(input.bg),
			underline_color: Color::LightMagenta, // DEBUG: This is intentionally set to a trash color as I do not plan to make use of it at this time
			modifier: Modifier::from_bits(input.modifier).unwrap_or(Modifier::empty()),
		}
	}
}
impl From<&Renderable> for ScreenCell {
	fn from(input: &Renderable) -> Self {
		ScreenCell {
			glyph: input.glyph.clone(),
			fg: input.fg,
			bg: input.bg,
			modifier: input.mods,
		}
	}
}
impl From<Tile> for ScreenCell {
	fn from(input: Tile) -> Self {
		ScreenCell {
			glyph: input.glyph.clone(),
			fg: input.fg,
			bg: input.bg,
			modifier: input.mods,
		}
	}
}
impl ScreenCell {
	pub fn empty() -> Self {
		ScreenCell {
			glyph: " ".to_string(),
			fg: 8,
			bg: 0,
			modifier: 0,
		}
	}
	pub fn blank() -> Self {
		ScreenCell {
			glyph: "".to_string(),
			fg: 0,
			bg: 0,
			modifier: 0,
		}
	}
	pub fn is_blank(&self) -> bool {
		self.glyph == *""
		//&& self.fg == 0
		//&& self.bg == 0
		//&& self.modifier == 0
	}
	pub fn glyph(&mut self, new_glyph: String) -> &mut Self {
		self.glyph = new_glyph;
		self
	}
	pub fn fg(&mut self, new_color: u8) -> &mut Self {
		self.fg = new_color;
		self
	}
	pub fn bg(&mut self, new_color: u8) -> &mut Self {
		self.bg = new_color;
		self
	}
	pub fn modifier(&mut self, new_mod: u16) -> &mut Self {
		self.modifier = new_mod;
		self
	}
}
/// Provides the camera update system for Bevy
pub fn camera_update_system(mut camera:   ResMut<CameraView>,
	                          model:        Res<Model>, // Provides the Terrain map
	                          s_query:      Query<(Entity, &Position, &Renderable), With<Facade>>,
	                          mut p_query:  Query<(Entity, &Position, &Renderable, &Viewshed, &Memory), With<Player>>,
	                          //b_query:      Query for blinkenlights: With<Blinken>
	                          //v_query:      Query for visual effects: With<Sparkle>
	                          // This last query is intended to cover *everything* that isn't in one of the above
	                          // It will need additional qualifiers applied as this method gets more complex
	                          //e_query:      Query<(Entity, &Position, &Renderable), (Without<Facade>, Without<Player>)>,
	                          e_query:      Query<(Entity, &Position, &Renderable), Without<Player>>,
) {
	// Bail out if any of the required objects aren't in place already
	if p_query.get_single_mut().is_err() { return; }
	let player = p_query.get_single_mut().unwrap();
	let world_map = &model.levels[player.1.z as usize];
	assert!(!camera.output.is_empty(), "camera.output has length 0!");
	assert!(!world_map.tiles.is_empty(), "world_map.tiles has length 0!");
	// Everything checks out OK, so get started on the update
	let camera_width = camera.width as usize;
	let screen_center = Position::create((camera_width / 2) as i32, camera.height / 2, 0);
	// These map_frame values together define the area of the map that we'll be polling
	let map_frame_ul = Position::create(player.1.x - screen_center.x, player.1.y - screen_center.y, 0);
	let map_frame_dr = Position::create(player.1.x + screen_center.x, player.1.y + screen_center.y, 0);
	// STAGE 1: update each map layer in the camera
	// Reset the map layers so we don't get overprinting
	// We don't have to worry about resetting the terrain map because all of it will be overwritten
	for entry in &mut camera.scenery { *entry = ScreenCell::blank(); }
	for entry in &mut camera.actors { *entry = ScreenCell::blank(); }
	for entry in &mut camera.blinken { *entry = ScreenCell::blank(); }
	//for entry in &mut camera.vfx { *entry = ScreenCell::blank(); } // -> stub
	// For every y-position in the map frame ...
	for (scr_y, map_y) in (map_frame_ul.y..map_frame_dr.y).enumerate() {
		// For every x-position in the map frame ...
		for (scr_x, map_x) in (map_frame_ul.x..map_frame_dr.x).enumerate() {
			debug!("* scr: {}, {}; map: {}, {}", scr_x, scr_y, map_x, map_y); // DEBUG: print the loop iteration values
			let scr_index = xy_to_index(scr_x, scr_y, camera_width); // Indexes into the camera's map of the screen
			let map_posn = Position::create(map_x, map_y, player.1.z); // handy container
			let map_index = world_map.to_index(map_x, map_y); // Indexes into the worldmap's tilemap
			let is_visible = player.3.visible_tiles.contains(&Point::new(map_x, map_y));
			let has_seen = if map_index < world_map.revealed_tiles.len() {
				world_map.revealed_tiles[map_index]
			} else {
				false
			};
			// Update the terrain map
			if map_x >= 0 && map_x < world_map.width as i32
			&& map_y >= 0 && map_y < world_map.height as i32
			&& (is_visible || has_seen)
			{
				camera.terrain[scr_index] = world_map.tiles[map_index].clone().into();
				if !is_visible { camera.terrain[scr_index].fg(8); }
			} else {
				camera.terrain[scr_index] = ScreenCell::blank(); // Painting this blank tile helps prevent artifacting
			}
			// Update the scenery map
			for scenery in s_query.iter() {
				if scenery.1.x == map_x && scenery.1.y == map_y {
					camera.scenery[scr_index] = scenery.2.into();
				}
			}
			// Update the actor map
			for actor in e_query.iter() {
				if *actor.1 == map_posn && is_visible {
					camera.actors[scr_index] = actor.2.into();
				}
			}
			// Update the actor map with the player's memories as well
			for memory in player.4.visual.iter() {
				if *memory.1 == map_posn && !is_visible {
					if let Ok(enty) = e_query.get(*memory.0) {
						camera.actors[scr_index] = enty.2.into();
						camera.actors[scr_index].fg(8);
					}
				}
			}
			// Paint the player onto the actor map
			if *player.1 == map_posn {
				camera.actors[scr_index] = player.2.into();
			}
			// TODO: Update the Blinkenlights map (persistent/cyclic effects)
			// Paint the targeting reticle onto the map if needed
			if camera.reticle != Position::INVALID {
				// TODO: Add some logic that will detect other entity positions (such as the player!) and choose
				//       a reticle shape that minimizes the number of entities who will be hidden by the points
				// TODO: Add a line-of-sight ruler that can show where the LOS is blocked with line coloration
				let ul_index = xy_to_index(camera.reticle.x as usize - 1, camera.reticle.y as usize - 1, camera_width);
				let ur_index = xy_to_index(camera.reticle.x as usize + 1, camera.reticle.y as usize - 1, camera_width);
				let dl_index = xy_to_index(camera.reticle.x as usize - 1, camera.reticle.y as usize + 1, camera_width);
				let dr_index = xy_to_index(camera.reticle.x as usize + 1, camera.reticle.y as usize + 1, camera_width);
				let ret_chars = camera.reticle_glyphs.clone();
				for (index, corner) in ret_chars.chars().enumerate() {
					match ret_chars.chars().count() {
						3 => { todo!(); /* TODO: impl logic for 3-point reticles */ }
						4 => {
							match index {
								0 => {camera.blinken[ul_index].glyph = corner.to_string(); camera.blinken[ul_index].fg = 11; camera.blinken[ul_index].bg = 8;}
								1 => {camera.blinken[ur_index].glyph = corner.to_string(); camera.blinken[ur_index].fg = 11; camera.blinken[ur_index].bg = 8;}
								2 => {camera.blinken[dl_index].glyph = corner.to_string(); camera.blinken[dl_index].fg = 11; camera.blinken[dl_index].bg = 8;}
								3 => {camera.blinken[dr_index].glyph = corner.to_string(); camera.blinken[dr_index].fg = 11; camera.blinken[dr_index].bg = 8;}
								_ => { }
							}
						}
						_ => { }
					}
				}
			}
			// TODO: Update the Sparkles map (the transitory visual effects)
		}
	}
	// STAGE 2: flatten the maps downward into a single map for the ratatui widget
	// Make a temp container of a vector of vectors where each inner vec's first element is a terrain tile
	let mut bucket: Vec<Vec<ScreenCell>> = camera.terrain.iter().map(|x| vec![x.clone()]).collect();
	// Make a pair-iterator out of the set of terrain tiles in the bucket, and each of the camera layer maps
	// If a non-blank tile is found in one of the maps, append it to the vec-of-vecs for that position
	for (target, input) in bucket.iter_mut().zip(camera.scenery.iter()) {
		if !input.is_blank() {
			target.push(input.clone());
		}
	}
	for (target, input) in bucket.iter_mut().zip(camera.actors.iter()) {
		if !input.is_blank() {
			target.push(input.clone());
		}
	}
	for (target, input) in bucket.iter_mut().zip(camera.blinken.iter()) {
		if !input.is_blank() {
			target.push(input.clone());
		}
	}
	// Build the camera map out of the stacked layers, choosing the 'highest'/last element in each vector
	for (index, cell) in bucket.iter().enumerate() {
		camera.output[index] = cell.iter().last().unwrap().clone();
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
