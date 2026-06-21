# Rename and Copy Fidelity Note

This note records the current diff/status fidelity baseline before any backend swap.

## Current Baseline

The current libgit2-backed implementation is covered by tests for:

- rename detection in file-history lookups
- copies remaining ordinary added changes in file-history lookups
- typechanges surfacing as `Deleted` in file-history lookups for the symlink swap fixture
- directory-like file names without an extension remaining ordinary file rows
- workdir status conflict rows for files that are not renames/copies

Relevant coverage lives in:

- `src/git/queries/diffs.rs`
- `src/git/queries/file_history.rs`
- `src/tests/git/queries/diffs.rs`
- `src/tests/git/queries/file_history.rs`
- `src/tests/app/draw/status.rs`
- `src/tests/app/draw/viewer.rs`

## Migration Shape

gitoxide will need to preserve the user-visible rename behavior first.
Copy detection is a lower-priority fidelity issue because the current UI does not present a dedicated copy marker.
Typechanges also remain collapsed for now because the UI only has the current status buckets.

## Behavioral Notes

- Rename detection is the important fidelity boundary for file history and status rendering.
- Copies are currently acceptable as ordinary added changes.
- Typechanges are currently acceptable as `Deleted` in the symlink swap fixture.
- Directory-like paths must remain treated as file paths when they are actual files.

## Test Gate

Before replacing any diff/file-history code:

- keep the rename, copy, typechange, and directory-like fixtures green
- keep the status and viewer tests that render those rows green
- do not swap any diff classification code until the same cases pass with the candidate backend
