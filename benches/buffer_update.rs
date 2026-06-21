mod fixtures;

use divan::{Bencher, black_box};
use fixtures::{BufferOp, apply_buffer_ops, buffer_checkpoint_fixture, buffer_linear_fixture, buffer_merge_fixture};
use guitar::core::buffer::Buffer;

fn main() {
    divan::main();
}

fn update_buffer(ops: &[BufferOp]) -> Buffer {
    black_box(apply_buffer_ops(ops))
}

#[divan::bench(sample_count = 100, sample_size = 100)]
fn buffer_update_linear_small(bencher: Bencher) {
    let fixture = buffer_linear_fixture(32);

    bencher.counter(divan::counter::ItemsCount::new(fixture.ops.len())).bench(|| black_box(update_buffer(&fixture.ops)));
}

#[divan::bench(sample_count = 100, sample_size = 100)]
fn buffer_update_merge_medium(bencher: Bencher) {
    let fixture = buffer_merge_fixture(12);

    bencher.counter(divan::counter::ItemsCount::new(fixture.ops.len())).bench(|| black_box(update_buffer(&fixture.ops)));
}

#[divan::bench(sample_count = 100, sample_size = 100)]
fn buffer_update_checkpoint_stress(bencher: Bencher) {
    let fixture = buffer_checkpoint_fixture(160);

    bencher.counter(divan::counter::ItemsCount::new(fixture.ops.len())).bench(|| black_box(update_buffer(&fixture.ops)));
}

#[divan::bench(sample_count = 100, sample_size = 100)]
fn buffer_window_replay_small(bencher: Bencher) {
    let fixture = buffer_linear_fixture(48);

    bencher.counter(divan::counter::ItemsCount::new(fixture.buffer.deltas.len())).bench(|| black_box(fixture.buffer.window(fixture.window_start, fixture.window_end)));
}

#[divan::bench(sample_count = 100, sample_size = 100)]
fn buffer_window_replay_medium(bencher: Bencher) {
    let fixture = buffer_merge_fixture(10);

    bencher.counter(divan::counter::ItemsCount::new(fixture.buffer.deltas.len())).bench(|| black_box(fixture.buffer.window(fixture.window_start, fixture.window_end)));
}

#[divan::bench(sample_count = 100, sample_size = 100)]
fn buffer_window_replay_stress(bencher: Bencher) {
    let fixture = buffer_checkpoint_fixture(180);

    bencher.counter(divan::counter::ItemsCount::new(fixture.buffer.deltas.len())).bench(|| black_box(fixture.buffer.window(fixture.window_start, fixture.window_end)));
}
