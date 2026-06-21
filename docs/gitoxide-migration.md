# Gitoxide Migration Boundary

This note tracks the staged backend migration from libgit2 to gitoxide.
The goal is to keep the UI stable while replacing one narrow Git surface at a time behind tests.

## Phase 0

Before any backend swap, the regression fixture matrix must stay green:

- `src/git/test_support.rs`
- `src/tests/git/test_support.rs`

Those fixtures cover the current behavior for unborn repositories, detached HEAD, linked worktrees, submodules, conflicts, tags, fetch/push flows, stash/reset, and large histories.

## Backend-Neutral Seam

The first seam lives in `src/git/repository.rs`.

It centralizes repository discovery and linked-worktree owner lookup so callers stop reaching for `git2::Repository::open` directly.

## Phase 1 Stays libgit2-Backed

The following surfaces stay on the current backend until their parity fixtures and adapter notes are ready:

- refs and graph roots
- status and diff
- checkout and reset
- sequencer operations
- linked worktrees
- submodules
- network callbacks and auth prompts

Linked worktrees are especially sensitive because the current app depends on shared common-dir behavior. If gitoxide parity is incomplete there, keep that seam on libgit2 in phase 1.

### Linked Worktree Decision

Keep linked worktree opening and owner lookup on libgit2 for the first staged backend.

gitoxide's public feature list covers worktree checkout and worktree stream support, but the linked-worktree common-dir behavior that guitar needs is not documented as a supported parity surface. Until that gap is verified and the matching API exists, the `git::repository::open_worktree_owner()` seam should continue to call libgit2.

## Migration Order

1. Repository open and discovery
2. Refs and graph roots
3. Status and diff
4. Checkout and reset
5. Sequencer operations
6. Linked worktrees
7. Submodules
8. Network callbacks

Checkout and reset details are tracked in [docs/gitoxide-checkout-reset.md](gitoxide-checkout-reset.md).
Sequencer details are tracked in [docs/gitoxide-sequencer.md](gitoxide-sequencer.md).
Status and diff details are tracked in [docs/gitoxide-status-diff.md](gitoxide-status-diff.md).
Rename and copy fidelity details are tracked in [docs/gitoxide-diff-fidelity.md](gitoxide-diff-fidelity.md).
Refs, remotes, and tag details are tracked in [docs/gitoxide-refs-remotes-tags.md](gitoxide-refs-remotes-tags.md).
Stash details are tracked in [docs/gitoxide-stash.md](gitoxide-stash.md).
Submodule details are tracked in [docs/gitoxide-submodules.md](gitoxide-submodules.md).
Network and auth details are tracked in [docs/gitoxide-network.md](gitoxide-network.md).

## Test Gate

Before each swap:

- add or extend fixtures for the exact surface
- keep the targeted surface tests green
- run the full test suite
- file a Beads issue if a behavior gap appears instead of widening the swap
