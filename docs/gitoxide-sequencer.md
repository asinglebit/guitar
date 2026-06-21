# Merge, Rebase, Cherry-Pick, and Revert Parity Note

This note records the current parity matrix for the sequencer-style operations.

## Current Baseline

The current libgit2-backed implementation is covered by tests for:

- clean merge completion
- fast-forward merge
- `merge.ff=only` rejection
- `merge.ff=false` merge-commit preference
- merge conflict stop and continue
- merge abort
- clean rebase completion
- rebase conflict stop and continue
- rebase abort
- clean cherry-pick completion
- cherry-pick conflict stop and continue
- cherry-pick abort
- clean revert completion
- revert conflict stop and continue
- revert abort
- in-progress operation routing through the app controller
- operation refusal when the worktree is dirty

Relevant test coverage lives in:

- `src/tests/git/actions/merging.rs`
- `src/tests/git/actions/rebasing.rs`
- `src/tests/git/actions/cherrypicking.rs`
- `src/tests/git/actions/reverting.rs`
- `src/tests/app/input/git.rs`

## gitoxide Surface Mapping

gitoxide publicly exposes merge-related and sequencing-related crates:

- `gix-merge` for blob/tree/commit merge primitives
- `gix-rebase` for rebase behavior
- `gix-sequencer` for sequencing workflows

The public checkout/writing primitive lives in `gix-worktree-state::checkout`, which is relevant because all of these workflows ultimately need to update the worktree after ref and index changes.

That suggests the eventual backend swap will likely be composed from:

- ref movement or ref transaction updates
- merge / rebase / sequencer primitives
- worktree checkout to materialize the final state
- controller-level routing for continue/abort/error handling

## Behavioral Notes

- Merge fast-forward and `merge.ff` policy handling remain controller responsibilities.
- Rebase, cherry-pick, and revert need to preserve the current conflict/continue/abort flow.
- Repository state detection is part of the app controller contract and should remain test-backed.
- No user-visible blocker has been identified yet, but the exact composition of gitoxide primitives still needs a prototype before any swap.

## Test Gate

Before replacing any of these operations:

- keep the existing action tests green
- keep the app-controller routing tests green
- add a prototype note for the exact gitoxide call sequence
- do not swap the implementation until merge/rebase/cherry-pick/revert all have surface-specific parity proof
