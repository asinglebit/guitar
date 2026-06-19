use super::*;
use crate::core::chunk::{Chunk, NONE};

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
    buffer.expire_lane_after_snapshot(merge.lane_idx);
    buffer.update(Chunk::commit(2, NONE, NONE));
    buffer.backup();

    let history = buffer.window(1, buffer.deltas.len());

    assert_eq!(history[2].len(), 3);
    assert_eq!(history[2][merge.lane_idx].alias, 6);
    assert_eq!(history[3].len(), 2);
    assert!(history[3].iter().all(|chunk| chunk.alias != 6));
}

#[test]
fn update_records_only_changed_parent_lanes() {
    let mut buffer = Buffer::default();

    buffer.update(Chunk::commit(3, 2, NONE));
    buffer.update(Chunk::commit(4, 99, NONE));
    let untouched = buffer.curr[1];

    buffer.update(Chunk::commit(2, NONE, NONE));

    assert_eq!(buffer.curr[1], untouched);
    assert!(!buffer.delta.ops.iter().any(|op| matches!(op, DeltaOp::Replace { index: 1, .. })));

    buffer.backup();
    let history = buffer.window(1, buffer.deltas.len());
    let latest = history.back().unwrap();

    assert_eq!(latest[0], Chunk::commit(2, NONE, NONE));
    assert_eq!(latest[1], untouched);
}
