# Checkout and Reset Parity Note

This note records the current checkout/reset baseline and the likely gitoxide mapping before any backend swap.

## Current Baseline

The libgit2-backed surface currently supports:

- detached checkout via `checkout_head`
- local branch checkout via `checkout_branch`
- remote branch materialization during checkout
- hard reset via `reset_to_commit`
- mixed reset via `reset_to_commit` with `ResetType::Mixed`
- path-level discard via `reset_file`

The new linked-worktree fixtures in:

- `src/tests/git/actions/checkout.rs`
- `src/tests/git/actions/resetting.rs`

prove those flows still work when the repository is opened from a linked worktree path.

## Likely gitoxide Mapping

The checkout side has a documented worktree-writing primitive in `gix-worktree-state::checkout`.
The public API is worktree/index oriented and uses `gix_worktree::stack::State::for_checkout()` under the hood.

That suggests the checkout replacement will likely split into:

- ref selection or HEAD movement
- worktree checkout via `gix-worktree-state`
- branch visibility refresh in the UI layer

Reset appears to be more composition-heavy.
I did not find a single documented one-shot reset API in the public gitoxide crates I checked, so reset will likely need to be expressed as:

- ref movement
- index rewrite
- optional worktree checkout or worktree preservation depending on reset type

## Behavioral Notes

- Hard reset should continue to move the checked-out ref and rewrite the worktree.
- Mixed reset should continue to move the checked-out ref while preserving the worktree contents.
- Path reset should continue to remove staged entries and restore the file from HEAD.
- Linked worktrees must keep using the common-dir owner seam until gitoxide owner/common-dir behavior is verified.

## Test Gate

Before replacing anything on this surface:

- keep the existing helper fixtures green
- keep the new linked-worktree checkout/reset tests green
- add a backend-specific prototype or adapter note for the exact gitoxide calls
- do not swap the UI behavior until the current behavior is locked down by tests
