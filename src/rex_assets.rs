// rex_assets.rs - provides methods for handling REXPaint files
use bracket_rex::prelude::*;
use bracket_embedding::prelude::*;
use bevy::ecs::prelude::*;

// add embedded_resource!s here
embedded_resource!(TEST_SHIP, "../resources/test_ship.xp");
#[derive(Resource)]
pub struct RexAssets {
	pub menu: XpFile
}
impl RexAssets {
	#[allow(clippy::new_without_default)]
	pub fn new() -> RexAssets {
		//this is where to link_resource! the above embeds
		link_resource!(TEST_SHIP, "../resources/test_ship.xp");
		RexAssets {
			menu: XpFile::from_resource("../resources/test_ship.xp").unwrap()
		}
	}
}

// EOF
