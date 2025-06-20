use bevy::prelude::*;
use section_color_postprocess::PostProcessPlugin;
use section_color_prepass::SectionTexturePhasePlugin;
use unlit_material::unlit_material_plugin;

use crate::rendering::{section_color_prepass::DrawSection, test_material::test_material_plugin};

pub mod section_color_postprocess;
pub mod section_color_prepass;
pub mod test_material;
pub mod unlit_material;

pub fn render_plugins(app: &mut App) {
    app.add_plugins((
        SectionTexturePhasePlugin,
        PostProcessPlugin,
        unlit_material_plugin,
        test_material_plugin,
    ));
    app.register_type::<DrawSection>();
}
