use super::*;
use crate::core::chunk::{Chunk, NONE};
use std::mem::size_of;

fn linear_history_buffer(total: u32) -> Buffer {
    let mut buffer = Buffer::default();
    for alias in 1..=total {
        buffer.update(Chunk::commit(alias, alias - 1, NONE));
    }
    buffer.backup();
    buffer
}

fn capped_buffer(limit: usize, aliases: std::ops::RangeInclusive<u32>) -> (Buffer, Vec<UpdateOutcome>) {
    let mut buffer = Buffer::with_lane_limit(limit);
    let outcomes = aliases.map(|alias| buffer.update(Chunk::commit(alias, 100 + alias, NONE))).collect();
    (buffer, outcomes)
}

#[test]
fn window_rebuilds_visible_range_from_delta_history() {
    let mut buffer = Buffer::default();

    buffer.update(Chunk::commit(2, 1, NONE));
    buffer.update(Chunk::commit(1, NONE, NONE));
    buffer.backup();

    let history = buffer.window(1, buffer.deltas.len());

    assert_eq!(history.len(), 2);
    assert_eq!(history.get(0).unwrap().len(), 1);
    assert_eq!(history.get(0).unwrap()[0].alias, 2);
    assert_eq!(history.get(0).unwrap()[0].parent_a, 1);
    assert_eq!(history.get(1).unwrap().len(), 1);
    assert_eq!(history.get(1).unwrap()[0].alias, 1);
    assert_eq!(history.get(1).unwrap()[0].parent_a, NONE);
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
    assert_eq!(window.get(0).unwrap()[0].alias, 3);
    assert_eq!(window.get(1).unwrap()[0].alias, 2);
    assert_eq!(window.get(2).unwrap()[0].alias, 1);
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

    assert_eq!(history.get(2).unwrap().len(), 3);
    assert_eq!(history.get(2).unwrap()[merge.lane.index].alias, 6);
    assert_eq!(history.get(3).unwrap().len(), 2);
    assert!(history.get(3).unwrap().iter().all(|chunk| chunk.alias != 6));
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
    let latest = history.last().unwrap();

    assert_eq!(latest[0], Chunk::commit(2, NONE, NONE));
    assert_eq!(latest[1], untouched);
}

#[test]
fn capped_buffer_bounds_history_and_records_overflow_delta() {
    let (mut buffer, outcomes) = capped_buffer(5, 1..=7);

    for update in &outcomes[5..] {
        assert_eq!(update.lane.index, 4);
        assert!(update.lane.is_flattened);
    }
    assert!(buffer.delta.ops.iter().any(|op| matches!(op, DeltaOp::Truncate { len: 5 } | DeltaOp::ReplaceAndTruncate { len: 5, .. })));
    assert!(!buffer.delta.ops.iter().any(|op| matches!(op, DeltaOp::Remove { index } if *index >= 5)));
    assert_eq!(buffer.delta.ops.iter().count(), 2);

    buffer.backup();
    let history = buffer.window(1, buffer.deltas.len());

    assert!(history.rows().iter().all(|snapshot| snapshot.len() <= 5));
    let latest = history.last().unwrap();
    assert_eq!(latest.len(), 5);
    assert_eq!(latest[4].alias, 7);
    assert!(latest[4].is_flattened);
}

#[test]
fn window_replays_many_op_delta_from_compacted_history() {
    let mut buffer = Buffer::default();

    buffer.backup();
    buffer.delta.ops.push(DeltaOp::Insert { index: 0, item: Chunk::commit(1, NONE, NONE) });
    buffer.delta.ops.push(DeltaOp::Insert { index: 1, item: Chunk::commit(2, NONE, NONE) });
    buffer.delta.ops.push(DeltaOp::Replace { index: 0, new: Chunk::commit(3, NONE, NONE) });
    buffer.delta.ops.push(DeltaOp::Remove { index: 1 });
    buffer.delta.ops.push(DeltaOp::Truncate { len: 1 });
    buffer.curr.push(Chunk::commit(3, NONE, NONE));
    buffer.backup();

    let history = buffer.window(1, buffer.deltas.len());

    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().len(), 1);
    assert_eq!(history.get(0).unwrap()[0], Chunk::commit(3, NONE, NONE));
}

#[test]
fn common_many_op_delta_stays_inline() {
    let mut ops = DeltaOps::default();

    ops.push(DeltaOp::Insert { index: 0, item: Chunk::commit(1, NONE, NONE) });
    ops.push(DeltaOp::Replace { index: 0, new: Chunk::commit(2, NONE, NONE) });
    ops.push(DeltaOp::Truncate { len: 1 });

    assert_eq!(ops.len(), 3);
    assert!(!ops.spilled());
}

#[test]
fn capped_buffer_keeps_normal_last_lane_palette_eligible_without_overflow() {
    let (buffer, _) = capped_buffer(5, 1..=5);

    assert_eq!(buffer.curr.len(), 5);
    assert_eq!(buffer.curr[4].alias, 5);
    assert!(!buffer.curr[4].is_flattened);
}

#[test]
fn shrink_to_fit_releases_overreserved_delta_capacity() {
    let mut buffer = Buffer::default();

    for alias in 1..=16 {
        buffer.update(Chunk::commit(alias, NONE, NONE));
    }
    buffer.backup();

    let capacity_before = buffer.deltas.capacity();
    buffer.shrink_to_fit();

    assert!(capacity_before > buffer.deltas.capacity());
    assert_eq!(buffer.deltas.capacity(), buffer.deltas.len());
    let history = buffer.window(1, buffer.deltas.len());
    assert_eq!(history.len(), 16);
}

#[test]
fn long_history_keeps_sparse_checkpoints_and_replays_late_window() {
    let buffer = linear_history_buffer(16_500);

    let start = 16_001;
    let full = buffer.window(1, buffer.deltas.len());
    let history = buffer.window(start, buffer.deltas.len());
    let expected = GraphHistory::from_rows(full.rows().iter().skip(start - 1).cloned());

    assert_eq!(buffer.checkpoints.len(), 2);
    assert_eq!(buffer.checkpoints[0].idx, 0);
    assert_eq!(buffer.checkpoints[1].idx, 16_384);
    assert_eq!(history.len(), 500);
    assert_eq!(history, expected);
}

#[test]
fn delta_op_chunks_keep_arena_allocations_bounded() {
    let mut buffer = Buffer::with_lane_limit(20);

    for alias in 1..=40_000 {
        buffer.update(Chunk::commit(alias, 100_000 + alias, NONE));
    }
    buffer.backup();

    let capacities = buffer.deltas.op_chunk_capacities();
    assert_eq!(capacities.len(), 1);
    assert!(capacities.iter().all(|capacity| *capacity <= 131_072));
}

#[test]
fn delta_ops_stay_compact_for_large_histories() {
    assert_eq!(size_of::<DeltaOp>(), 24);
}

#[test]
fn delta_spans_stay_packed_for_large_histories() {
    assert_eq!(size_of::<DeltaSpan>(), 8);
}

#[test]
fn transient_lane_expiration_reuses_scratch_storage() {
    let mut buffer = Buffer::default();

    for lane in 0..16 {
        buffer.expire_lane_after_snapshot(lane);
    }
    let capacity = buffer.transient_lanes.capacity();

    buffer.update(Chunk::commit(1, NONE, NONE));

    assert!(buffer.transient_lanes.is_empty());
    assert_eq!(buffer.transient_lanes.capacity(), capacity);
}
