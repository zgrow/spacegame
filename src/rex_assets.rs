// rex_assets.rs - provides methods for handling REXPaint files
use bracket_rex::prelude::*;
use bracket_embedding::prelude::*;
use bevy::ecs::prelude::*;

// add embedded_resource!s here
// leaving the TEST_SHIP stuff here as a case example
//embedded_resource!(TEST_SHIP, "../resources/test_ship_v2.xp");
embedded_resource!(PAUSE_GRAPHIC, "../resources/big_pause.xp");
#[derive(Resource)]
pub struct RexAssets {
	//pub test_map: XpFile,
	pub pause_banner: XpFile
}
impl RexAssets {
	#[allow(clippy::new_without_default)]
	pub fn new() -> RexAssets {
		//this is where to link_resource! the above embeds
		//link_resource!(TEST_SHIP, "../resources/test_ship.xp");
		link_resource!(PAUSE_GRAPHIC, "../resources/big_pause.xp");
		RexAssets {
			//test_map: XpFile::from_resource("../resources/test_ship.xp").unwrap(),
			pause_banner: XpFile::from_resource("../resources/big_pause.xp").unwrap()
		}
	}
}

// EOF
