# Network and Auth Parity Note

This note records the current fetch/push/auth baseline and the migration shape before any backend swap.

## Current Baseline

The current libgit2-backed implementation is covered by tests for:

- default-remote fetch
- fetch of remote heads and tags
- push of the current branch
- push of all local tags
- delete-remote-branch flows
- HTTPS credential prompt, retry, and cache reuse
- SSH credential prompt, retry, and cache reuse
- SSH default-key selection order
- auth modal routing and cancellation behavior in the app controller

Relevant coverage lives in:

- `src/git/actions/fetching.rs`
- `src/git/actions/pushing.rs`
- `src/git/auth.rs`
- `src/tests/git/test_support.rs`
- `src/tests/git/auth.rs`
- `src/tests/app/input/git.rs`
- `src/tests/app/draw/modals/auth.rs`

## Migration Shape

The network surface should be treated as two subphases:

1. fetch/push plumbing and ref updates
2. auth callback and prompt behavior

The first subphase can move the transport and ref-update paths while the UI contract stays fixed.
The second subphase can replace the auth callback path once the prompt, cache, and retry flow are fully covered by tests.

## Behavioral Notes

- HTTPS auth should continue to cache username/password pairs by URL and username.
- SSH auth should continue to cache passphrases by URL, username, and key path.
- SSH prompts should remain secret-only in the UI.
- Default SSH key lookup should continue to prefer `id_ed25519`, then `id_ecdsa`, then `id_rsa`.
- The migration should preserve the current retry and cancellation behavior until the candidate backend is proven by tests.

## Test Gate

Before replacing any network/auth code:

- keep the current fetch, push, and auth tests green
- add or keep a prototype note for the exact gitoxide callback and transport mapping
- do not swap prompt behavior or retry flow until the candidate backend passes the same fixtures
