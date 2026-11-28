use std::error::Error;

use unitas_runtime::{
    test_read_path_buffer, test_write_path_buffer, unitas_runtime_destroy,
    unitas_runtime_navmesh_agent_ensure, unitas_runtime_navmesh_agent_get_destination,
    unitas_runtime_navmesh_agent_has, unitas_runtime_navmesh_agent_has_path,
    unitas_runtime_navmesh_agent_set_destination, unitas_runtime_navmesh_agent_warp,
    unitas_runtime_navmesh_calculate_path, unitas_runtime_navmesh_sample_position,
    unitas_runtime_navmesh_set_surface_triangles, unitas_runtime_new, unitas_runtime_path_buffer,
    unitas_runtime_terrain_ensure, unitas_runtime_terrain_get_heights,
    unitas_runtime_terrain_get_resolution, unitas_runtime_terrain_has,
    unitas_runtime_terrain_sample_height, unitas_runtime_terrain_set_heights,
    unitas_runtime_terrain_set_size,
};

fn write_f64_le(bytes: &mut [u8], offset: usize, value: f64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_f64_le(bytes: &[u8], offset: usize) -> f64 {
    let mut raw = [0u8; 8];
    raw.copy_from_slice(&bytes[offset..offset + 8]);
    f64::from_le_bytes(raw)
}

fn read_f32_le(bytes: &[u8], offset: usize) -> f32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    f32::from_le_bytes(raw)
}

/// Authored floor: 4 cells in a row on the y=0 plane, each split into two
/// triangles, spanning x in [0, 4], z in [0, 1].
fn floor_triangles() -> Vec<[f64; 3]> {
    let mut vertices = Vec::new();
    for index in 0..4usize {
        let x = index as f64;
        for [ax, az, bx, bz, cx, cz] in [
            [x, 0.0, x, 1.0, x + 1.0, 0.0],
            [x + 1.0, 0.0, x, 1.0, x + 1.0, 1.0],
        ] {
            vertices.push([ax, 0.0, az]);
            vertices.push([bx, 0.0, bz]);
            vertices.push([cx, 0.0, cz]);
        }
    }
    vertices
}

#[test]
fn runtime_abi_terrain_heights_round_trip_and_sample() -> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);
    let entity = 42u64;

    assert_eq!(unitas_runtime_terrain_ensure(handle, 0, 3), -2);
    assert_eq!(unitas_runtime_terrain_ensure(handle, entity, 3), 0);
    assert_eq!(unitas_runtime_terrain_has(handle, entity), 1);
    assert_eq!(unitas_runtime_terrain_get_resolution(handle, entity), 3);
    assert_eq!(unitas_runtime_terrain_get_resolution(handle, 777), -3);
    assert_eq!(
        unitas_runtime_terrain_set_size(handle, entity, 100.0, 50.0, 100.0),
        0
    );

    // Write a 3x3 block of authored heights at normalized value 0.5
    // (world height 25 with size y = 50).
    let heights = [0.5f32; 9];
    let mut height_bytes = vec![0u8; heights.len() * 4];
    for (index, value) in heights.iter().enumerate() {
        height_bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    let buffer = unitas_runtime_path_buffer(handle, height_bytes.len() as u64);
    assert_ne!(buffer, 0);
    test_write_path_buffer(handle, &height_bytes)?;
    assert_eq!(
        unitas_runtime_terrain_set_heights(handle, entity, 0, 0, 3, 3, buffer),
        0
    );

    let mut read_back = vec![0u8; height_bytes.len()];
    assert_eq!(
        unitas_runtime_terrain_get_heights(handle, entity, 0, 0, 3, 3, buffer),
        0
    );
    test_read_path_buffer(handle, &mut read_back)?;
    for index in 0..9 {
        assert_eq!(read_f32_le(&read_back, index * 4), 0.5);
    }

    // The terrain is centered at the origin, so the world-space sample at the
    // center reads back the authored world height (0.5 * 50 = 25).
    let out_buffer = unitas_runtime_path_buffer(handle, 8);
    assert_ne!(out_buffer, 0);
    assert_eq!(
        unitas_runtime_terrain_sample_height(handle, entity, 0.0, 0.0, 0.0, out_buffer),
        0
    );
    let mut sample_bytes = [0u8; 8];
    test_read_path_buffer(handle, &mut sample_bytes)?;
    assert_eq!(f64::from_le_bytes(sample_bytes), 25.0);
    assert_eq!(
        unitas_runtime_terrain_sample_height(handle, 777, 0.0, 0.0, 0.0, out_buffer),
        -3
    );

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}

#[test]
fn runtime_abi_navmesh_surface_sample_and_path_match_authored_geometry()
-> Result<(), Box<dyn Error>> {
    let handle = unitas_runtime_new();
    assert_ne!(handle, 0);

    // Push 8 authored triangles (80 bytes each: 3 vertices + area) and build
    // the navmesh surface from them.
    // Each triangle is an 80-byte record: three f64 vertices plus an f64 area.
    let triangles = floor_triangles();
    assert_eq!(triangles.len(), 24);
    let mut triangle_bytes = vec![0u8; triangles.len() / 3 * 80];
    for (index, vertex) in triangles.iter().enumerate() {
        let base = index / 3 * 80 + index % 3 * 24;
        write_f64_le(&mut triangle_bytes, base, vertex[0]);
        write_f64_le(&mut triangle_bytes, base + 8, vertex[1]);
        write_f64_le(&mut triangle_bytes, base + 16, vertex[2]);
    }
    let buffer = unitas_runtime_path_buffer(handle, triangle_bytes.len() as u64);
    assert_ne!(buffer, 0);
    test_write_path_buffer(handle, &triangle_bytes)?;
    assert_eq!(
        unitas_runtime_navmesh_set_surface_triangles(handle, buffer, 8),
        0
    );

    // Nearest-point query above the authored floor snaps onto it.
    let out_buffer = unitas_runtime_path_buffer(handle, 64);
    assert_ne!(out_buffer, 0);
    let found = unitas_runtime_navmesh_sample_position(handle, 0.5, 0.1, 0.5, 2.0, -1, out_buffer);
    assert_eq!(found, 1);
    let mut hit_bytes = [0u8; 64];
    test_read_path_buffer(handle, &mut hit_bytes)?;
    assert_eq!(read_f64_le(&hit_bytes, 0), 0.5);
    assert_eq!(read_f64_le(&hit_bytes, 8), 0.0);
    assert_eq!(read_f64_le(&hit_bytes, 16), 0.5);
    assert_eq!(read_f64_le(&hit_bytes, 24), 0.0);
    assert_eq!(read_f64_le(&hit_bytes, 32), 1.0);
    assert_eq!(read_f64_le(&hit_bytes, 40), 0.0);
    assert_eq!(read_f64_le(&hit_bytes, 48), 0.1);
    assert_eq!(read_f64_le(&hit_bytes, 56), 1.0); // area_bit(0) == 1

    // A point far from the authored surface finds nothing.
    assert_eq!(
        unitas_runtime_navmesh_sample_position(handle, 50.0, 5.0, 50.0, 1.0, -1, out_buffer),
        0
    );

    // Path query across the floor: status Complete, corners start and end at
    // the authored endpoints.
    let status =
        unitas_runtime_navmesh_calculate_path(handle, 0.5, 0.0, 0.5, 3.5, 0.0, 0.5, -1, out_buffer);
    assert_eq!(status, 0);
    let mut header = [0u8; 8];
    test_read_path_buffer(handle, &mut header)?;
    assert_eq!(i32::from_le_bytes(header[0..4].try_into()?), 0);
    let corner_count = i32::from_le_bytes(header[4..8].try_into()?);
    assert!(corner_count >= 2, "path corners: {corner_count}");
    let mut path_bytes = vec![0u8; 8 + corner_count as usize * 24];
    test_read_path_buffer(handle, &mut path_bytes)?;
    assert_eq!(read_f64_le(&path_bytes, 8), 0.5);
    assert_eq!(read_f64_le(&path_bytes, 16), 0.0);
    assert_eq!(read_f64_le(&path_bytes, 24), 0.5);
    let last = 8 + (corner_count as usize - 1) * 24;
    assert!((read_f64_le(&path_bytes, last) - 3.5).abs() < 1e-6);
    assert_eq!(read_f64_le(&path_bytes, last + 8), 0.0);
    assert_eq!(read_f64_le(&path_bytes, last + 16), 0.5);

    // Agent flow on the authored surface: warp on, destination reachable.
    let agent = 7u64;
    assert_eq!(unitas_runtime_navmesh_agent_ensure(handle, agent), 0);
    assert_eq!(unitas_runtime_navmesh_agent_has(handle, agent), 1);
    assert_eq!(
        unitas_runtime_navmesh_agent_warp(handle, agent, 0.5, 0.0, 0.5),
        1
    );
    assert_eq!(
        unitas_runtime_navmesh_agent_set_destination(handle, agent, 3.5, 0.0, 0.5),
        0
    );
    assert_eq!(unitas_runtime_navmesh_agent_has_path(handle, agent), 1);

    let dest_buffer = unitas_runtime_path_buffer(handle, 24);
    assert_ne!(dest_buffer, 0);
    assert_eq!(
        unitas_runtime_navmesh_agent_get_destination(handle, agent, dest_buffer),
        0
    );
    let mut dest_bytes = [0u8; 24];
    test_read_path_buffer(handle, &mut dest_bytes)?;
    assert_eq!(read_f64_le(&dest_bytes, 0), 3.5);
    assert_eq!(read_f64_le(&dest_bytes, 8), 0.0);
    assert_eq!(read_f64_le(&dest_bytes, 16), 0.5);

    assert_eq!(unitas_runtime_destroy(handle), 0);
    Ok(())
}
