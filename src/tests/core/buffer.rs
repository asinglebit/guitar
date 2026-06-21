use super::*;
use crate::core::chunk::{Chunk, NONE};

fn compressed_parents(buffer: &Buffer) -> Vec<u32> {
    buffer.compressed_parents.iter().copied().collect()
}

#[test]
fn window_rebuilds_visible_range_from_delta_history() {
    let mut buffer = Buffer::default();

    buffer.update(Chunk::commit(2, 1, NONE));
    buffer.update(Chunk::commit(1, NONE, NONE));
    buffer.backup();

    let history = buffer.window(1, buffer.deltas.len());

    assert_eq!(history.len(), 2);
    assert_eq!(history[0].len(), 1);
    assert_eq!(history[0][0].alias, 2);
    assert_eq!(history[0][0].parent_a, 1);
    assert_eq!(history[1].len(), 1);
    assert_eq!(history[1][0].alias, 1);
    assert_eq!(history[1][0].parent_a, NONE);
}

#[test]
fn window_does_not_mutate_current_graph_state() {
    let mut buffer = Buffer::default();

    buffer.update(Chunk::commit(3, 2, NONE));
    buffer.update(Chunk::commit(2, 1, NONE));
    buffer.update(Chunk::commit(1, NONE, NONE));
    buffer.backup();

    let before = buffer.curr.clone();
    let window = buffer.window(1, buffer.deltas.len());

    assert_eq!(buffer.curr, before);
    assert_eq!(window.len(), 3);
    assert_eq!(window[0][0].alias, 3);
    assert_eq!(window[1][0].alias, 2);
    assert_eq!(window[2][0].alias, 1);
}

#[test]
fn planned_merger_splits_lane_before_replacing_first_parent() {
    let mut buffer = Buffer::default();

    buffer.update(Chunk::commit(3, 1, 2));
    buffer.merger(3);
    buffer.update(Chunk::commit(1, NONE, NONE));

    assert!(buffer.mergers.is_empty());
    assert_eq!(buffer.curr.len(), 2);
    assert_eq!(buffer.curr[0].alias, 1);
    assert_eq!(buffer.curr[0].parent_a, NONE);
    assert_eq!(buffer.curr[1].alias, 3);
    assert_eq!(buffer.curr[1].parent_a, 2);
    assert_eq!(buffer.curr[1].parent_b, NONE);
}

#[test]
fn transient_lane_survives_one_snapshot_then_expires() {
    let mut buffer = Buffer::default();

    buffer.update(Chunk::commit(4, 2, NONE));
    buffer.update(Chunk::commit(5, 3, NONE));
    let merge = buffer.update(Chunk::commit(6, 2, 3));
    buffer.expire_lane_after_snapshot(merge.lane.index);
    buffer.update(Chunk::commit(2, NONE, NONE));
    buffer.backup();

    let history = buffer.window(1, buffer.deltas.len());

    assert_eq!(history[2].len(), 3);
    assert_eq!(history[2][merge.lane.index].alias, 6);
    assert_eq!(history[3].len(), 2);
    assert!(history[3].iter().all(|chunk| chunk.alias != 6));
}

#[test]
fn update_records_only_changed_parent_lanes() {
    let mut buffer = Buffer::default();

    buffer.update(Chunk::commit(3, 2, NONE));
    buffer.update(Chunk::commit(4, 99, NONE));
    let untouched = buffer.curr[1].clone();

    buffer.update(Chunk::commit(2, NONE, NONE));

    assert_eq!(buffer.curr[1], untouched);
    assert!(!buffer.delta.ops.iter().any(|op| matches!(op, DeltaOp::Replace { index: 1, .. })));

    buffer.backup();
    let history = buffer.window(1, buffer.deltas.len());
    let latest = history.back().unwrap();

    assert_eq!(latest[0], Chunk::commit(2, NONE, NONE));
    assert_eq!(latest[1], untouched);
}

#[test]
fn capped_buffer_never_returns_snapshots_wider_than_lane_limit() {
    let mut buffer = Buffer::with_lane_limit(5);

    for alias in 1..=7 {
        let update = buffer.update(Chunk::commit(alias, 100 + alias, NONE));
        if alias > 5 {
            assert_eq!(update.lane.index, 4);
            assert!(update.lane.is_flattened);
        }
    }
    buffer.backup();

    let history = buffer.window(1, buffer.deltas.len());

    assert!(history.iter().all(|snapshot| snapshot.len() <= 5));
    let latest = history.back().unwrap();
    assert_eq!(latest.len(), 5);
    assert_eq!(latest[4].alias, 7);
    assert!(latest[4].is_flattened);
}

#[test]
fn capped_buffer_keeps_normal_last_lane_palette_eligible_without_overflow() {
    let mut buffer = Buffer::with_lane_limit(5);

    for alias in 1..=5 {
        buffer.update(Chunk::commit(alias, 100 + alias, NONE));
    }

    assert_eq!(buffer.curr.len(), 5);
    assert_eq!(buffer.curr[4].alias, 5);
    assert!(!buffer.curr[4].is_flattened);
}

#[test]
fn capped_buffer_preserves_collapsed_parent_scanlines() {
    let mut buffer = Buffer::with_lane_limit(3);

    buffer.update(Chunk::commit(10, 110, NONE));
    buffer.update(Chunk::commit(11, 111, NONE));
    buffer.update(Chunk::commit(12, 112, NONE));
    buffer.update(Chunk::commit(13, 113, NONE));

    assert_eq!(buffer.curr.len(), 3);
    assert_eq!(buffer.curr[2].alias, 13);
    assert_eq!(buffer.curr[2].parent_a, 113);
    assert_eq!(buffer.curr[2].parent_b, 112);
    assert!(buffer.curr[2].is_flattened);
    assert_eq!(compressed_parents(&buffer), vec![113, 112]);

    buffer.backup();
    let latest = buffer.window(1, buffer.deltas.len()).back().unwrap().clone();
    assert_eq!(latest.compressed_parents.iter().copied().collect::<Vec<_>>(), vec![113, 112]);
}

#[test]
fn capped_buffer_consumes_only_reached_collapsed_parent() {
    let mut buffer = Buffer::with_lane_limit(3);

    buffer.update(Chunk::commit(10, 110, NONE));
    buffer.update(Chunk::commit(11, 111, NONE));
    buffer.update(Chunk::commit(12, 112, NONE));
    buffer.update(Chunk::commit(13, 113, NONE));

    let update = buffer.update(Chunk::commit(112, 212, NONE));

    assert_eq!(update.lane.index, 2);
    assert!(update.lane.is_flattened);
    assert!(!update.started_lane);
    assert_eq!(buffer.curr[2].alias, 112);
    assert_eq!(buffer.curr[2].parent_a, 212);
    assert_eq!(buffer.curr[2].parent_b, 113);
    assert!(buffer.curr[2].is_flattened);
    assert_eq!(compressed_parents(&buffer), vec![113]);
}

#[test]
fn capped_buffer_starts_new_span_markers_without_losing_unresolved_compressed_parents() {
    let mut buffer = Buffer::with_lane_limit(3);

    buffer.update(Chunk::commit(10, 110, NONE));
    buffer.update(Chunk::commit(11, 111, NONE));
    buffer.update(Chunk::commit(12, 112, NONE));
    buffer.update(Chunk::commit(13, 113, NONE));
    buffer.update(Chunk::commit(20, 120, NONE));

    assert_eq!(buffer.curr.len(), 3);
    assert_eq!(buffer.curr[2].alias, 20);
    assert_eq!(buffer.curr[2].parent_a, 120);
    assert_eq!(buffer.curr[2].parent_b, 120);
    assert!(buffer.curr[2].is_flattened);
    assert_eq!(compressed_parents(&buffer), vec![113, 112, 120]);
}
