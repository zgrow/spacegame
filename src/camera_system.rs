/// camera_system.rs
/// Provides implementation for the CameraView component, including refresh/update logic

use crate::components::*;
use crate::map::*;
use bevy::ecs::system::*;
use bevy::ecs::event::EventReader;
use bevy::ecs::query::Without;
use bracket_geometry::prelude::*;

/// Represents a 'flattened' view of the Map's layers, with all entities and effects painted in,
/// such that it can be read by the Viewport object when it comes time to render the view
#[derive(Resource)]
pub struct CameraView {
	pub map: Vec<Tile>,
	pub width: i32,
	pub height: i32,
}
/// Provides an abstraction to the Viewport widget with hooks into Bevy's systems for updates
impl CameraView {
	pub fn new(new_width: i32, new_height: i32) -> Self {
		Self {
			map: vec![default_tile(); (new_width * new_height) as usize],
			width: new_width,
			height: new_height
		}
	}
	pub fn set_dims(&mut self, new_width: i32, new_height: i32) {
		// NOTE: include a sanity check here that actually examines the dims prior to resize
		// if the resize is required, then probably safest to wipe the whole thing...
		// either way, make sure that the CameraView gets an update before next render call
		self.width = new_width;
		self.height = new_height;
		let new_size = (self.width * self.height) as usize;
		if self.map.len() != new_size {
			self.map = vec![default_tile(); new_size];
		}
	}
}
/// Provides the update system for Bevy
pub fn camera_update_sys(mut camera: ResMut<CameraView>,
	                     renderables: Query<(&Position, &Renderable), Without<Player>>,
	                     model: Res<Model>,
	                     mut q_player: Query<(&Player, &Viewshed, &Position, &Renderable)>,
	                     mut _ereader: EventReader<GameEvent>,
) {
	/* UPDATE STRATEGY
	 * Each layer in the list gets applied in the order it appears: this 'flattens' the
	 * abstraction into a single 2D plane that can be rendered on the Viewport
	 * The Tile abstraction is setup to convert almost directly into tui-rs::buffer::Cells
	 * though be aware that the Tiles retain more data than the Cells do
	 * This is the priority stack that determines which layers are drawn over others:
	 * Structuring like this allows us to prevent redrawing a Tile many times
	 * 1 Animation FX   (not impl)
	 * 2 Scenery FX     (not impl)
	 * 3 Player Entity  -
	 * 4 NPC Entities    \
	 * 5 Props            } Covered by Renderables list
	 * 6 Furniture       /  (only Player impl at this time)
	 * 7 Scenery        -
	 * 8 Terrain        Map::Vec<TileType>
	 */
	/* METHOD
	 * Given self.width, self.height = the Viewport's size ('screen' size)
	 *      self.map = the output result, a vector of Tiles, which must be filled,
	 *      screen_x/y refers to Cell coords within the Viewport's buffer,
	 *      target_x/y refers to coords within the World context,
	 *      t_min.x/y and t_max.x/y describe the 2D plane of possible World coordinates that we
	 *          need to inquire about to draw the entire Viewport
	 * 1    Obtain the player's position (== ppos)
	 * 2    Obtain the screen size (== self.width/height)
	 * 3    Calculate the centerpoint of the viewscreen: screen.size / 2
	 * 4    Obtain the min/max x,y coords relative to the player's position:
	 *          (player_x - center_x, player_y - center_y)
	 * 5    Begin drawing the map:
	 *      let screen_y = 1                        //starting at first screen row...
	 *      for target_y in min.y to max.y {        //iter on all map rows...
	 *          let screen_x = 1                    //starting at first screen col...
	 *          for target_x in min.x to max.x {    //iter on all map cols...
	 *              if target_x and target_y are within the map bounds: [ie 0 <= n < max_dim]
	 *                  cameraview[index].tile = [layer renderables as above]
	 *              else
	 *                  cameraview[index].tile = [fallback tile, ie blank/spacefield]
	 *              screen_x++                      //move to next col
	 *          }
	 *          screen_y++                          //move to next row
	 *      }
	 */
	let player = q_player.get_single_mut().unwrap();
	//eprintln!("Updating CameraView from map at z-level {}", player.2.z); // DEBUG:
	let world_map = &model.levels[player.2.z as usize];
	// Absolutely positively do not try to do this if the camera or map are empty
	assert!(camera.map.len() != 0, "camera.map has length 0!");
	assert!(world_map.tiles.len() != 0, "world_map.tiles has length 0!");
	let centerpoint = Position{x: camera.width / 2, y: camera.height / 2, z: 0};
	let minima = Position{x: player.2.x - centerpoint.x, y: player.2.y - centerpoint.y, z: 0};
	let maxima = Position{x: player.2.x + centerpoint.x, y: player.2.y + centerpoint.y, z: 0};
	let mut screen_y = 0;
	for target_y in minima.y..maxima.y {
		let mut screen_x = 0;
		for target_x in minima.x..maxima.x {
			// We are iterating on target_x/y AND screen_x/y
			// Update the world_map and buf indices at the same time to avoid confusion
			let map_index = world_map.to_index(target_x, target_y);
			let buf_index = xy_to_index(screen_x, screen_y, camera.width);
			let mut new_tile = default_tile();
			// Check for an existing tile in the world_map
			// Don't use map_index to perform the bounds check:
			// it'll map to ANY valid index, too many false positives
			// IF the target_x/y produces a valid world_map coordinate...
			if target_x >= 0 && target_x < world_map.width
			&& target_y >= 0 && target_y < world_map.height
			&& world_map.revealed_tiles[map_index] { // and if the tile's been seen before...
				// ... THEN put together the displayed tile from various input sources:
				// First, obtain the background
				new_tile = world_map.tiles[map_index].clone();
				if target_x == player.2.x && target_y == player.2.y {
					// If this tile is where the player's standing, draw the player and move on
					new_tile.glyph = player.3.glyph.clone();
					new_tile.fg = player.3.fg;
					new_tile.bg = player.3.bg;
					new_tile.mods = "".to_string();
				} else if player.1.visible_tiles.contains(&Point::new(target_x, target_y)) {
					// Else, render everything the player can see that isn't the player itself
					if !&renderables.is_empty() {
						for (posn, rendee) in &renderables {
							if (posn.x, posn.y, posn.z) == (target_x, target_y, player.2.z) {
								new_tile.glyph = rendee.glyph.clone();
								new_tile.fg = rendee.fg;
								new_tile.bg = rendee.bg;
								new_tile.mods = "".to_string();
							}
						}
					}
					// TODO: check for a scenery effect
					// TODO: check for an animation effect
				} else {
					// Otherwise, just assume this is a real-but-not-visible tile and recolor it
					new_tile.fg = 8;
					new_tile.bg = 0;
					new_tile.mods = "".to_string();
				}
			} else {
				// ... ELSE just make it a background tile (ie starfield)
				new_tile.glyph = "░".to_string();
			}
			camera.map[buf_index] = new_tile;
			screen_x += 1;
		}
		screen_y += 1;
	}
}
/// Prototype that returns a 'blank' kind of tile. Planned to be replaced with logic that draw a
/// starfield background, when there is time to implement such.
fn default_tile() -> Tile {
	Tile {
		ttype: TileType::Floor,
		glyph: "#".to_string(),
		fg: 8,
		bg: 0,
		mods: "".to_string()
	}
}

// EOF
