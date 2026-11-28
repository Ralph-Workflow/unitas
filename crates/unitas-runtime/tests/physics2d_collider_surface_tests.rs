use std::path::PathBuf;

use unitas_runtime::{
    test_read_path_buffer, unitas_runtime_body2d_create, unitas_runtime_body2d_set_velocity,
    unitas_runtime_body2d_velocity, unitas_runtime_collider2d_create,
    unitas_runtime_collider2d_create_edge, unitas_runtime_collider2d_create_polygon,
    unitas_runtime_collider2d_set_composite, unitas_runtime_collider2d_set_material,
    unitas_runtime_destroy, unitas_runtime_new, unitas_runtime_overlap_point2d,
    unitas_runtime_path_buffer, unitas_runtime_query_buffer_2d, unitas_runtime_raycast2d,
    unitas_runtime_tick, unitas_runtime_tilemap_collider2d_build,
};

fn write_f32_to_path_buffer(handle: u64, values: &[f32]) -> *const f32 {
    let byte_len = values.len() * 4;
    let buf_addr = unitas_runtime_path_buffer(handle, byte_len as u64);
    assert_ne!(buf_addr, 0);
    let mut bytes = Vec::with_capacity(byte_len);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    assert!(unitas_runtime::test_write_path_buffer(handle, &bytes).is_ok());
    buf_addr as usize as *const f32
}

fn write_bytes_to_path_buffer(handle: u64, bytes: &[u8]) -> *const u8 {
    let buf_addr = unitas_runtime_path_buffer(handle, bytes.len() as u64);
    assert_ne!(buf_addr, 0);
    assert!(unitas_runtime::test_write_path_buffer(handle, bytes).is_ok());
    buf_addr as usize as *const u8
}

fn read_body_velocity(handle: u64, entity: u64) -> [f32; 2] {
    let buf_addr = unitas_runtime_path_buffer(handle, 16);
    assert_ne!(buf_addr, 0);
    assert_eq!(unitas_runtime_body2d_velocity(handle, entity, buf_addr), 0);
    let mut raw = [0u8; 8];
    assert!(test_read_path_buffer(handle, &mut raw).is_ok());
    let vx = f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    let vy = f32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
    [vx, vy]
}

fn step_world_2d(handle: u64, steps: usize) {
    assert_eq!(unitas_runtime_tick(handle, 0.0), 0);
    for _ in 0..steps {
        assert_eq!(unitas_runtime_tick(handle, 0.02), 0);
    }
}

#[test]
fn polygon_and_edge_colliders_created_via_abi_participate_in_2d_simulation_contact_and_queries() {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);

    let poly_entity = 10u64;
    let edge_entity = 20u64;
    let ball_poly_entity = 30u64;
    let ball_edge_entity = 40u64;

    assert_eq!(
        unitas_runtime_body2d_create(handle, poly_entity, 0.0, 0.0, 0.0, 1.0, 0.0, 2, 0, 1, 0, 0,),
        0
    );
    let poly_points: [f32; 8] = [-2.0, -0.5, 2.0, -0.5, 2.0, 0.5, -2.0, 0.5];
    let poly_ptr = write_f32_to_path_buffer(handle, &poly_points);
    assert_eq!(
        unitas_runtime_collider2d_create_polygon(
            handle,
            poly_entity,
            poly_entity,
            poly_ptr,
            4,
            0.0,
            0.0,
            0,
            0,
        ),
        0
    );

    assert_eq!(
        unitas_runtime_body2d_create(handle, edge_entity, 6.0, 0.0, 0.0, 1.0, 0.0, 2, 0, 1, 0, 0,),
        0
    );
    let edge_points: [f32; 6] = [-2.0, 0.5, 0.0, 0.5, 2.0, 0.5];
    let edge_ptr = write_f32_to_path_buffer(handle, &edge_points);
    assert_eq!(
        unitas_runtime_collider2d_create_edge(
            handle,
            edge_entity,
            edge_entity,
            edge_ptr,
            3,
            0.0,
            0.0,
            0,
            0,
        ),
        0
    );

    let _ = unitas_runtime_query_buffer_2d(handle, 256);
    assert_eq!(
        unitas_runtime_raycast2d(handle, 0.0, 3.0, 0.0, -1.0, 5.0, u32::MAX, 0),
        1
    );
    assert_eq!(
        unitas_runtime_raycast2d(handle, 6.0, 3.0, 0.0, -1.0, 5.0, u32::MAX, 0),
        1
    );

    assert_eq!(
        unitas_runtime_body2d_create(
            handle,
            ball_poly_entity,
            0.0,
            1.2,
            0.0,
            1.0,
            1.0,
            0,
            0,
            1,
            0,
            0,
        ),
        0
    );
    assert_eq!(
        unitas_runtime_collider2d_create(
            handle,
            ball_poly_entity,
            ball_poly_entity,
            1,
            0.0,
            0.0,
            0.0,
            0.0,
            0.25,
            0,
            0,
        ),
        0
    );

    assert_eq!(
        unitas_runtime_body2d_create(
            handle,
            ball_edge_entity,
            6.0,
            1.2,
            0.0,
            1.0,
            1.0,
            0,
            0,
            1,
            0,
            0,
        ),
        0
    );
    assert_eq!(
        unitas_runtime_collider2d_create(
            handle,
            ball_edge_entity,
            ball_edge_entity,
            1,
            0.0,
            0.0,
            0.0,
            0.0,
            0.25,
            0,
            0,
        ),
        0
    );

    step_world_2d(handle, 60);

    assert_eq!(
        unitas_runtime_overlap_point2d(handle, 0.0, 0.75, u32::MAX),
        1,
        "dynamic circle resting on polygon collider top face must overlap (0.0, 0.75)"
    );
    assert_eq!(
        unitas_runtime_overlap_point2d(handle, 6.0, 0.75, u32::MAX),
        1,
        "dynamic circle resting on edge collider chain must overlap (6.0, 0.75)"
    );

    let short_poly_ptr = write_f32_to_path_buffer(handle, &poly_points[..4]);
    assert_eq!(
        unitas_runtime_collider2d_create_polygon(
            handle,
            poly_entity,
            poly_entity,
            short_poly_ptr,
            2,
            0.0,
            0.0,
            0,
            0,
        ),
        -2
    );
    let short_edge_ptr = write_f32_to_path_buffer(handle, &edge_points[..2]);
    assert_eq!(
        unitas_runtime_collider2d_create_edge(
            handle,
            edge_entity,
            edge_entity,
            short_edge_ptr,
            1,
            0.0,
            0.0,
            0,
            0,
        ),
        -2
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
}

#[test]
fn physics2d_composite_outline_merges_two_abutting_box_colliders_and_slides_smoothly_across_seam() {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);

    let comp_entity = 100u64;
    let left_box_entity = 101u64;
    let right_box_entity = 102u64;
    let slider_entity = 103u64;

    assert_eq!(
        unitas_runtime_body2d_create(handle, comp_entity, 0.0, 0.0, 0.0, 1.0, 0.0, 2, 0, 1, 0, 0,),
        0
    );

    assert_eq!(
        unitas_runtime_collider2d_create(
            handle,
            left_box_entity,
            comp_entity,
            0,
            -1.0,
            0.0,
            2.0,
            1.0,
            0.0,
            0,
            0,
        ),
        0
    );
    assert_eq!(
        unitas_runtime_collider2d_create(
            handle,
            right_box_entity,
            comp_entity,
            0,
            1.0,
            0.0,
            2.0,
            1.0,
            0.0,
            0,
            0,
        ),
        0
    );

    assert_eq!(
        unitas_runtime_collider2d_set_composite(handle, left_box_entity, comp_entity, 1),
        0
    );
    assert_eq!(
        unitas_runtime_collider2d_set_composite(handle, right_box_entity, comp_entity, 1),
        0
    );
    assert_eq!(
        unitas_runtime_collider2d_set_material(handle, comp_entity, 0.0, 0.0),
        0
    );

    let _ = unitas_runtime_query_buffer_2d(handle, 256);
    assert_eq!(
        unitas_runtime_raycast2d(handle, -3.0, 0.0, 1.0, 0.0, 10.0, u32::MAX, 0),
        1,
        "a horizontal ray across both abutting boxes must hit only the single merged composite collider"
    );
    assert_eq!(
        unitas_runtime_overlap_point2d(handle, 0.0, 0.0, u32::MAX),
        1,
        "the seam at x=0 must belong to exactly one merged composite collider"
    );

    assert_eq!(
        unitas_runtime_body2d_create(
            handle,
            slider_entity,
            -1.2,
            0.75,
            0.0,
            1.0,
            1.0,
            0,
            0,
            1,
            0,
            0,
        ),
        0
    );
    assert_eq!(
        unitas_runtime_collider2d_create(
            handle,
            slider_entity,
            slider_entity,
            0,
            0.0,
            0.0,
            0.5,
            0.5,
            0.0,
            0,
            0,
        ),
        0
    );
    assert_eq!(
        unitas_runtime_collider2d_set_material(handle, slider_entity, 0.0, 0.0),
        0
    );
    assert_eq!(
        unitas_runtime_body2d_set_velocity(handle, slider_entity, 4.0, 0.0),
        0
    );

    assert_eq!(unitas_runtime_tick(handle, 0.0), 0);
    let mut max_vy_delta_across_seam = 0.0f32;
    let mut prev_vy = read_body_velocity(handle, slider_entity)[1];

    for _ in 0..40 {
        assert_eq!(unitas_runtime_tick(handle, 0.02), 0);
        let vel = read_body_velocity(handle, slider_entity);
        let vy_delta = (vel[1] - prev_vy).abs();
        if vy_delta > max_vy_delta_across_seam {
            max_vy_delta_across_seam = vy_delta;
        }
        prev_vy = vel[1];
    }

    assert_eq!(
        unitas_runtime_overlap_point2d(handle, 1.8, 0.75, u32::MAX),
        1,
        "dynamic body starting at x=-1.2 with vx=4.0 over 0.8s must slide smoothly past x=0 seam to x~2.0"
    );
    assert!(
        max_vy_delta_across_seam <= 0.05,
        "vertical velocity delta across the merged composite seam must be <= 0.05 u/s, got {max_vy_delta_across_seam}"
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
}

#[test]
fn tilemap_collider_build_from_fixture_feeds_composite_and_collides_with_dynamic_body() {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);

    let temp_dir =
        std::env::temp_dir().join(format!("unitas_u1_tilemap_fixture_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let tile_guid = "11112222333344445555666677778888";
    let tile_path = temp_dir.join("GroundTile.asset");
    let tile_meta_path = temp_dir.join("GroundTile.asset.meta");
    let tilemap_path: PathBuf = temp_dir.join("LevelTilemap.asset");

    let tile_yaml = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!114 &11400000\nTile:\n  m_Name: GroundTile\n  m_Sprite: {fileID: 21300000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}\n  m_ColliderType: 2\n";
    let tile_meta_yaml = format!("fileFormatVersion: 2\nguid: {tile_guid}\n");
    let tilemap_yaml = format!(
        "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1839735485 &1839735485\nTilemap:\n  m_Name: LevelTilemap\n  m_TileAssetArray:\n  - m_RefCount: 3\n    m_Data: {{fileID: 11400000, guid: {tile_guid}, type: 2}}\n    m_TileAsset: {{fileID: 11400000, guid: {tile_guid}, type: 2}}\n  m_Tiles:\n  - m_X: -1\n    m_Y: 0\n    m_Z: 0\n    m_TileIndex: 0\n  - m_X: 0\n    m_Y: 0\n    m_Z: 0\n    m_TileIndex: 0\n  - m_X: 1\n    m_Y: 0\n    m_Z: 0\n    m_TileIndex: 0\n"
    );

    assert!(std::fs::write(&tile_path, tile_yaml).is_ok());
    assert!(std::fs::write(&tile_meta_path, tile_meta_yaml).is_ok());
    assert!(std::fs::write(&tilemap_path, tilemap_yaml).is_ok());

    let tilemap_entity = 200u64;
    let composite_entity = 200u64;
    let faller_entity = 201u64;

    let path_str = tilemap_path.to_string_lossy().into_owned();
    let path_ptr = write_bytes_to_path_buffer(handle, path_str.as_bytes());
    let build_rc = unitas_runtime_tilemap_collider2d_build(
        handle,
        tilemap_entity,
        composite_entity,
        path_ptr,
        path_str.len() as u32,
    );
    let _ = std::fs::remove_dir_all(&temp_dir);
    assert_eq!(build_rc, 0);

    let _ = unitas_runtime_query_buffer_2d(handle, 128);
    assert_eq!(
        unitas_runtime_overlap_point2d(handle, 0.5, 0.5, u32::MAX),
        1,
        "merged composite tilemap collider must cover cell [0, 0] at (0.5, 0.5)"
    );
    assert_eq!(
        unitas_runtime_overlap_point2d(handle, -0.5, 0.5, u32::MAX),
        1,
        "merged composite tilemap collider must cover cell [-1, 0] at (-0.5, 0.5)"
    );
    assert_eq!(
        unitas_runtime_overlap_point2d(handle, 1.5, 0.5, u32::MAX),
        1,
        "merged composite tilemap collider must cover cell [1, 0] at (1.5, 0.5)"
    );

    assert_eq!(
        unitas_runtime_body2d_create(
            handle,
            faller_entity,
            0.5,
            2.0,
            0.0,
            1.0,
            1.0,
            0,
            0,
            1,
            0,
            0,
        ),
        0
    );
    assert_eq!(
        unitas_runtime_collider2d_create(
            handle,
            faller_entity,
            faller_entity,
            0,
            0.0,
            0.0,
            0.5,
            0.5,
            0.0,
            0,
            0,
        ),
        0
    );

    step_world_2d(handle, 80);

    assert_eq!(
        unitas_runtime_overlap_point2d(handle, 0.5, 1.25, u32::MAX),
        1,
        "dynamic box (half_y=0.25) falling onto composite tilemap top (y=1.0) must rest at y~1.25"
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
}

#[test]
fn collider2d_set_material_updates_friction_and_bounciness_and_changes_rebound_velocity() {
    let run_drop = |bounciness: f32, friction: f32| -> (f32, f32) {
        let handle = unitas_runtime_new();
        assert_ne!(handle, 0);

        let floor_entity = 300u64;
        let ball_entity = 301u64;

        assert_eq!(
            unitas_runtime_body2d_create(
                handle,
                floor_entity,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                2,
                0,
                1,
                0,
                0,
            ),
            0
        );
        assert_eq!(
            unitas_runtime_collider2d_create(
                handle,
                floor_entity,
                floor_entity,
                0,
                0.0,
                0.0,
                10.0,
                1.0,
                0.0,
                0,
                0,
            ),
            0
        );
        assert_eq!(
            unitas_runtime_collider2d_set_material(handle, floor_entity, friction, bounciness),
            0
        );

        assert_eq!(
            unitas_runtime_body2d_create(
                handle,
                ball_entity,
                0.0,
                0.7,
                0.0,
                1.0,
                0.0,
                0,
                0,
                1,
                0,
                0,
            ),
            0
        );
        assert_eq!(
            unitas_runtime_collider2d_create(
                handle,
                ball_entity,
                ball_entity,
                1,
                0.0,
                0.0,
                0.0,
                0.0,
                0.25,
                0,
                0,
            ),
            0
        );
        assert_eq!(
            unitas_runtime_collider2d_set_material(handle, ball_entity, friction, bounciness),
            0
        );

        assert_eq!(
            unitas_runtime_body2d_set_velocity(handle, ball_entity, 4.0, -6.0),
            0
        );
        step_world_2d(handle, 1);
        let vel = read_body_velocity(handle, ball_entity);
        assert_eq!(unitas_runtime_destroy(handle), 0);
        (vel[0], vel[1])
    };

    let (vx_low_f, vy_no_bounce) = run_drop(0.0, 0.0);
    let (vx_high_f, vy_bouncy) = run_drop(0.9, 0.8);

    assert!(
        vy_bouncy > vy_no_bounce + 4.0,
        "bounciness=0.9 must produce a much larger upward rebound velocity than bounciness=0.0: bouncy vy={vy_bouncy}, flat vy={vy_no_bounce}"
    );
    assert!(
        vx_high_f < vx_low_f - 1.0,
        "friction=0.8 must slow horizontal contact velocity compared to friction=0.0: high_f vx={vx_high_f}, low_f vx={vx_low_f}"
    );
}
