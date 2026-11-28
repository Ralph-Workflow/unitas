use std::fs;
use std::path::{Path, PathBuf};
use unitas_asset::meta;
use unitas_asset::{
    parse_material, parse_material_with_guids, GuidIndex, Material, MaterialError, MetaError,
};
use unitas_render::gpu::Renderer;
use unitas_render::{
    cube, look_at, Camera, DirectionalLight, Image, Mat4, MeshInstance, RenderScene, Vec3,
};

use super::render_tests::shared_renderer;

fn decode_bmp_pixel(bmp: &[u8], width: u32, height: u32, x: u32, y: u32) -> [u8; 3] {
    let row_bytes = (width as usize) * 3;
    let row_stride = (row_bytes + 3) & !3;
    let bmp_row = (height - 1 - y) as usize;
    let offset = 54 + bmp_row * row_stride + (x as usize) * 3;
    if offset + 2 < bmp.len() {
        [bmp[offset + 2], bmp[offset + 1], bmp[offset]]
    } else {
        [0, 0, 0]
    }
}

fn assert_material_uses_the_wood_texture(
    material: &Material,
    wood_guid: &str,
    guids: &GuidIndex,
    expected_tex_path: &Path,
) {
    assert_eq!(material.shader_guid, "0000000000000000f000000000000000");
    assert_eq!(material.color, [1.0, 0.5, 0.25, 1.0]);
    assert_eq!(material.main_tex_guid.as_deref(), Some(wood_guid));
    assert_eq!(
        material
            .main_tex_guid
            .as_deref()
            .and_then(|guid| guids.get(guid)),
        Some(expected_tex_path)
    );
}

fn render_material_cube(
    renderer: &Renderer,
    rgb: [f32; 3],
) -> Result<(Image, Image), Box<dyn std::error::Error>> {
    let mesh = cube(1.0);
    let camera = Camera::perspective(60.0, 1.0, 0.1, 100.0);
    let light = DirectionalLight {
        direction: Vec3::new(0.0, 0.0, -1.0),
        color: [1.0, 1.0, 1.0],
        intensity: 1.0,
    };
    let width = 64;
    let height = 64;
    let view = look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let instances = [MeshInstance {
        mesh: &mesh,
        model: Mat4::identity(),
        color: rgb,
    }];
    let img1 = renderer.render_offscreen(
        &instances,
        &camera,
        view,
        &light,
        [0.0, 0.0, 0.0],
        (width, height),
    )?;
    let scene = RenderScene {
        camera: &camera,
        view,
        light: &light,
        background: [0.0, 0.0, 0.0],
    };
    let img2 = renderer.render_offscreen_scene(&instances, scene, width, height)?;
    Ok((img1, img2))
}

fn assert_bmp_round_trip(
    img1: &Image,
    img2: &Image,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_bmp: PathBuf = std::env::temp_dir().join(format!(
        "unitas_render_e2e_material_{}.bmp",
        std::process::id()
    ));
    img1.write_bmp(&tmp_bmp)?;
    let bmp_bytes = fs::read(&tmp_bmp)?;
    let _ = fs::remove_file(&tmp_bmp);

    assert_eq!(bmp_bytes, img1.to_bmp_bytes());
    assert_eq!(bmp_bytes, img2.to_bmp_bytes());
    let bmp_center = decode_bmp_pixel(&bmp_bytes, width, height, width / 2, height / 2);
    assert_eq!(bmp_center, [255, 128, 64]);
    Ok(())
}

#[test]
fn material_to_pixel_end_to_end_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    let project_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../unitas-asset/tests/fixtures/project");
    let mut guids = GuidIndex::build(&project_dir).map_err(|e: MetaError| e.to_string())?;

    let expected_tex_path = project_dir.join("Textures").join("Wood.png");
    let wood_meta_path = project_dir.join("Textures").join("Wood.png.meta");
    let wood_guid = meta::parse_meta_file(&wood_meta_path).map_err(|e: MetaError| e.to_string())?;
    assert_eq!(guids.get(&wood_guid), Some(expected_tex_path.as_path()));

    let material_path = project_dir.join("Material.mat");
    let material = parse_material(&material_path).map_err(|e: MaterialError| e.to_string())?;
    assert_material_uses_the_wood_texture(&material, &wood_guid, &guids, &expected_tex_path);

    guids
        .insert(
            "0000000000000000f000000000000000".to_string(),
            project_dir.join("Shaders").join("Standard.shader"),
        )
        .map_err(|e: MetaError| e.to_string())?;
    let material_with_shader = parse_material_with_guids(&material_path, &guids)
        .map_err(|e: MaterialError| e.to_string())?;
    assert_eq!(
        material_with_shader.shader_name,
        Some("Standard".to_string())
    );

    let renderer = shared_renderer()?;
    let rgb = [material.color[0], material.color[1], material.color[2]];
    let (img1, img2) = render_material_cube(&renderer, rgb)?;

    let silhouette_1 = img1.pixels.iter().filter(|&&px| px != [0, 0, 0]).count();
    let silhouette_2 = img2.pixels.iter().filter(|&&px| px != [0, 0, 0]).count();
    assert!(silhouette_1 > 0);
    assert_eq!(silhouette_1, silhouette_2);
    assert_eq!(img1.get_pixel(32, 32), [255, 128, 64]);

    assert_bmp_round_trip(&img1, &img2, 64, 64)?;

    Ok(())
}
