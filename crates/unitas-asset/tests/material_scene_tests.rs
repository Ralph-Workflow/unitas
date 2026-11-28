use std::error::Error;
use std::path::PathBuf;

use unitas_asset::meta::GuidIndex;
use unitas_asset::scene::assemble_scene_file_with_index;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE_ROOT: &str = "../../samples/marble-rush";
const SAMPLE_SCENE: &str = "Assets/Scenes/MarbleRush.unity";
const SAMPLE_PICKUP_MATERIAL: &str = "Assets/Materials/Pickup.mat";
const SAMPLE_PICKUP_TEXTURE: &str = "Assets/Textures/Pickup.png";
const SAMPLE_PICKUP_NORMAL_TEXTURE: &str = "Assets/Textures/PickupNormal.png";
const SAMPLE_PICKUP_EMISSION_TEXTURE: &str = "Assets/Textures/PickupEmission.png";

fn sample_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAMPLE_ROOT)
}

#[test]
fn scene_renderer_carries_the_resolved_material_properties() -> TestResult {
    let root = sample_root();
    let index = GuidIndex::build(&root)?;
    let scene_path = root.join(SAMPLE_SCENE);
    let assembled = assemble_scene_file_with_index(&scene_path, &index)?;

    let renderer = assembled
        .render
        .renderers
        .iter()
        .find(|renderer| {
            renderer
                .material_path
                .as_deref()
                .is_some_and(|path| path.ends_with("Pickup.mat"))
        })
        .ok_or("no renderer uses the Pickup material")?;

    assert_eq!(
        renderer.material_path.as_deref(),
        Some(root.join(SAMPLE_PICKUP_MATERIAL).as_path())
    );
    assert_eq!(
        renderer.albedo_tex_path.as_deref(),
        Some(root.join(SAMPLE_PICKUP_TEXTURE).as_path())
    );
    assert_eq!(
        renderer.normal_tex_path.as_deref(),
        Some(root.join(SAMPLE_PICKUP_NORMAL_TEXTURE).as_path())
    );
    assert_eq!(
        renderer.emission_tex_path.as_deref(),
        Some(root.join(SAMPLE_PICKUP_EMISSION_TEXTURE).as_path())
    );
    assert_eq!(renderer.metallic, Some(0.0));
    assert_eq!(renderer.smoothness, Some(0.6));
    assert_eq!(renderer.emission_color, Some([0.25, 1.0, 0.9, 1.0]));
    assert_eq!(renderer.material_color, [0.18, 0.77, 0.71]);
    Ok(())
}
