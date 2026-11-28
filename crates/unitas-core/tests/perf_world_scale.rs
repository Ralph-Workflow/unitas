use std::error::Error;
use std::time::Instant;

use unitas_core::{ChangeKind, CoreError, EntityId, SceneChange, SceneGraph};

type TestResult = Result<(), Box<dyn Error>>;

const GAME_OBJECT_COUNT: usize = 100_000;
const SECTOR_COUNT: usize = 100;
const OBJECTS_PER_SECTOR: usize = GAME_OBJECT_COUNT / SECTOR_COUNT;
const CHURN_COUNT: usize = 10_000;
const REPARENT_BATCH: usize = 50_000;
const NAME_SAMPLE_STRIDE: usize = GAME_OBJECT_COUNT / 1_000;
const WORLD_SCALE_CEILING_MS: f64 = 6000.0;

fn build_world(graph: &mut SceneGraph) -> Result<Vec<EntityId>, CoreError> {
    let root = graph.add_entity("WorldRoot");
    let mut sectors = Vec::with_capacity(SECTOR_COUNT);
    for index in 0..SECTOR_COUNT {
        let sector = graph.add_entity(format!("Sector_{index}"));
        graph.attach(sector, root)?;
        sectors.push(sector);
    }
    let mut ids = Vec::with_capacity(GAME_OBJECT_COUNT);
    for index in 0..GAME_OBJECT_COUNT {
        let id = graph.add_entity(format!("GameObject_{index}"));
        graph.attach(id, sectors[index / OBJECTS_PER_SECTOR])?;
        ids.push(id);
    }
    Ok(ids)
}

fn assert_world_lookups(graph: &SceneGraph, ids: &[EntityId]) -> TestResult {
    for (index, id) in ids.iter().enumerate() {
        let entity = graph
            .entity(*id)
            .ok_or_else(|| format!("GameObject {index} vanished from the world"))?;
        if entity.id != *id {
            return Err(format!("GameObject {index} resolved to {}", entity.id).into());
        }
    }
    for index in (0..GAME_OBJECT_COUNT).step_by(NAME_SAMPLE_STRIDE) {
        let entity = graph
            .entity(ids[index])
            .ok_or_else(|| format!("GameObject {index} vanished from the world"))?;
        if entity.name != format!("GameObject_{index}") {
            return Err(format!("GameObject {index} carries the name {}", entity.name).into());
        }
    }
    Ok(())
}

fn churn_world(graph: &mut SceneGraph, ids: &[EntityId]) -> TestResult {
    for (index, stale) in ids.iter().copied().enumerate().take(CHURN_COUNT) {
        graph.mark_destroyed(stale)?;
        graph.sweep_destroyed();
        let fresh = graph.add_entity(format!("Churn_{index}"));
        if graph.entity(stale).is_some() {
            return Err(format!("the destroyed handle {stale} still resolves").into());
        }
        let entity = graph
            .entity(fresh)
            .ok_or_else(|| format!("the churned handle {fresh} vanished"))?;
        if entity.id != fresh {
            return Err(format!("the churned handle {fresh} resolved to {}", entity.id).into());
        }
    }
    Ok(())
}

fn count_changes(changes: &[SceneChange]) -> (usize, usize, usize, usize) {
    let mut created = 0usize;
    let mut reparent_changes = 0usize;
    let mut destroyed = 0usize;
    let mut transformed = 0usize;
    for change in changes {
        match change.kind {
            ChangeKind::Created => created += 1,
            ChangeKind::Reparented => reparent_changes += 1,
            ChangeKind::Destroyed => destroyed += 1,
            ChangeKind::TransformChanged => transformed += 1,
        }
    }
    (created, reparent_changes, destroyed, transformed)
}

fn assert_world_state(
    graph: &mut SceneGraph,
    ids: &[EntityId],
    group: EntityId,
    changes: &[SceneChange],
    counts: (usize, usize, usize, usize),
) {
    let (created, reparent_changes, destroyed, transformed) = counts;
    assert_eq!(graph.len(), GAME_OBJECT_COUNT + SECTOR_COUNT + 2);
    assert_eq!(graph.children(group).len(), REPARENT_BATCH);
    assert_eq!(graph.parent(ids[CHURN_COUNT]), Some(group));
    assert_ne!(graph.parent(ids[GAME_OBJECT_COUNT - 1]), Some(group));
    assert!(graph.parent(ids[GAME_OBJECT_COUNT - 1]).is_some());
    assert_eq!(created, GAME_OBJECT_COUNT + CHURN_COUNT + SECTOR_COUNT + 2);
    assert_eq!(
        reparent_changes,
        GAME_OBJECT_COUNT + REPARENT_BATCH + SECTOR_COUNT
    );
    assert_eq!(destroyed, CHURN_COUNT);
    assert_eq!(transformed, 0);
    assert!(changes
        .iter()
        .any(|change| change.kind == ChangeKind::Reparented
            && change.entity == ids[CHURN_COUNT]
            && change.parent == Some(group)));
    assert!(graph.take_changes().is_empty());
}

#[test]
#[ignore = "perf measurement runs in the make verify-perf lane"]
fn perf_world_scale_holds_a_hundred_thousand_game_object_world_within_the_ceiling() -> TestResult {
    let started = Instant::now();
    let mut graph = SceneGraph::new();
    let ids = build_world(&mut graph)?;
    let built = started.elapsed();

    let lookup_started = Instant::now();
    assert_world_lookups(&graph, &ids)?;
    let looked_up = lookup_started.elapsed();

    let churn_started = Instant::now();
    churn_world(&mut graph, &ids)?;
    let churned = churn_started.elapsed();

    let reparent_started = Instant::now();
    let group = graph.add_entity("ReparentedGroup");
    for id in ids.iter().skip(CHURN_COUNT).take(REPARENT_BATCH) {
        graph.attach(*id, group)?;
    }
    let reparented = reparent_started.elapsed();

    let drain_started = Instant::now();
    let changes = graph.take_changes();
    let drained = drain_started.elapsed();
    let value_ms = started.elapsed().as_secs_f64() * 1000.0;

    let counts = count_changes(&changes);
    assert_world_state(&mut graph, &ids, group, &changes, counts);

    println!(
        "world_scale_100k world: {GAME_OBJECT_COUNT} GameObjects in {SECTOR_COUNT} sectors, \
         build {built:?}, lookup {looked_up:?}, churn {churned:?}, reparent {reparented:?}, \
         drain {drained:?}"
    );
    println!(
        "measurement=world_scale_100k value_ms={value_ms:.3} ceiling_ms={WORLD_SCALE_CEILING_MS:.3}"
    );
    if value_ms > WORLD_SCALE_CEILING_MS {
        return Err(format!(
            "world_scale_100k measured {value_ms:.3}ms against the {WORLD_SCALE_CEILING_MS:.3}ms ceiling"
        )
        .into());
    }
    Ok(())
}
