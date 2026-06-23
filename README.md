<div align="center">
  <pre>
    </br>
                              :GG~        .?Y.                                
    ....        ..      ..   .....      . ^BG: ..       .....                 
 .7555YY7JP^   ~PJ     ~PJ  ?YY5PP~    7YY5BGYYYYJ.   J555YY557.              
.5B?.  :JBB~   !#5     !#5  ...PB~     ...^BG:....    ~:.   .7#5           :^^
7#5     .GB~   !B5     !B5     PB~        :BG.        .~7??J?JBG:      .~JPPPY
?#Y      PB~   !B5     !B5     PB~        :BG.       7GP7~^^^!BG:     ~5GY!:. 
^GB~    7BB~   ^BG.   .YB5     5#7        :BB:       P#!     JBG:    ^GG7     
 ^5G5JJYJPB~    JBP???YYB5     ^5GYJJ?.    7GPJ???.  ~PGJ77?5J5B!    JG5      
   .^~^..GB:     :~!!~. ^^       :~~~~      .^~~~~    .^!!!~. .^:    JG5      
 .?!^^^!5G7                                                          YB5      
 .!?JJJ?!:                                                           75?      
  </br>
terminal based cross-platform git client 
made with ♡
  </pre>
</div>

`guita╭`, distributed as `guitar`, is a Rust terminal UI for working with Git history from a topology-first point of view. It is built with `ratatui`, `crossterm`, and `libgit2`, and is designed for exploring, filtering, and operating on large repositories without leaving the terminal.

`guita╭` lets you instantly travel to prehistoric times in BIG repositores (500k+ commits)

![guitar screenshot](https://github.com/user-attachments/assets/177dbf13-b9ad-480e-a1be-71a333454a44)

## Contents

- [Status And Warning](#status-and-warning)
- [Demo](#demo)
- [Requirements](#requirements)
- [Install](#install)
- [Run](#run)
- [Mental Model](#mental-model)
- [Interface Sections](#interface-sections)
- [Navigation](#navigation)
- [File Search](#file-search)
- [Inputs And Keymaps](#inputs-and-keymaps)
- [Git Operations](#git-operations)
- [Authentication](#authentication)
- [Settings](#settings)
- [Persistence](#persistence)
- [Configuration Files](#configuration-files)
- [Development](#development)
- [Known Limitations](#known-limitations)
- [Roadmap](#roadmap)
- [Screenshots](#screenshots)

## Status And Warning

This is a hobby project with sharp Git tools. It can stage, unstage, commit, force checkout, force push, delete branches, reset, stash, pop, drop, rebase, merge, cherry-pick, revert, prune worktrees, update submodules, sync submodule URLs, and discard file changes.

Use it carefully on important repositories. Keep backups, understand what the selected row and focused pane mean before using action mode, and report issues when behavior is surprising.

## Demo

the complete rundown of features of the `v1.0.0`:

[https://m.youtube.com/watch?v=0oyqviuKFXI](https://m.youtube.com/watch?v=0oyqviuKFXI)

Older recording of the `v0.1.12` feature set and some technical breakdown of the core walker:

[https://www.youtube.com/watch?v=oERA8MYlHjQ](https://www.youtube.com/watch?v=oERA8MYlHjQ)

The recording is still useful as a tour. This README and the in-app settings/help view are the current source of truth for shortcuts and behavior.

## Requirements

- Rust and Cargo when building from source.
- A non-bare Git repository. Bare repositories are not supported by the UI.
- `user.name` and `user.email` configured in Git before opening a repository. `guitar` reads these on repository load and uses them when creating commits.
- Terminal mouse support for pane dragging and wheel scrolling.
- SSH or HTTPS credentials for private remotes when using network operations.

Set identity globally:

```bash
git config --global user.name "Your Name"
git config --global user.email "you@example.com"
```

Or set identity per repository:

```bash
git config user.name "Your Name"
git config user.email "you@example.com"
```

## Install

Prebuilt binaries are published on the releases page:

Important! Im not bothering myself with config migrations. So if you are trying out a new version, make sure to run `guitar --reset`. You might want to backup your `.config/guitar` folder before doing so. This is a temporary measure because im lazy.

[https://github.com/asinglebit/guitar/releases](https://github.com/asinglebit/guitar/releases)

Build from source:

```bash
git clone https://github.com/asinglebit/guitar.git
cd guitar
cargo build --release
```

The release binary is written to:

```bash
target/release/guitar
```

## Run

Run from inside a repository, a repository subdirectory, or with an explicit path:

```bash
guitar
guitar ../path/to/repository
```

The first non-flag argument is treated as the repository path. If no path is provided, `.` is used. The path is canonicalized and then resolved to the Git repository root when possible.

Meta flags:

```bash
guitar --version
guitar -v
guitar --reset
```

`--version` and `-v` print the version and exit. `--reset` deletes the saved `guitar` config directory, then starts the app with regenerated defaults.

If the path cannot be opened as a Git repository, `guitar` falls back to the splash screen and shows saved recent repositories.

## Mental Model

`guitar` is built around a few core ideas:

- The graph is the primary view. Commits, refs, stashes, worktrees, and optional HEAD reflog entries are projected onto one topology-oriented list.
- Row `0` is a synthetic uncommitted-work row above `HEAD`. It represents staged files, unstaged files, and conflicts in the working tree.
- Commit history loads incrementally on a background worker. The app becomes usable while more history is still being walked.
- Focus controls what keys operate on. The same key can scroll the graph, a side pane, a status pane, settings, a modal, or the file viewer depending on focus.
- Scope navigation is horizontal: `h` widens outward, `l` narrows inward.
- Dangerous commands are gated behind action mode. By default, press `Ctrl+a`, then the action key.

## Interface Sections

### Splash

The splash screen appears when no repository is open or when you back out of the graph. It lists recent repositories from `recent.json`.

- `Enter` or `l` opens the selected recent repository.
- `d` removes the selected recent repository from `recent.json`.
- `Shift+K` and `Shift+J` move the selected recent repository up or down.
- `Esc` returns to the graph when a repository is already loaded.
- `q` exits.

### Graph

The graph is the central history view. It can render:

- The synthetic uncommitted-work row.
- Commits in topology order.
- Local and remote branch labels.
- Lightweight and annotated tags resolved to commits.
- Stash commits placed near their base commits.
- Worktree badges for commits checked out in main or linked worktrees.
- Optional HEAD reflog labels and roots.
- Optional abbreviated SHA column.

Graph row details are loaded by window, so large repositories can stay responsive.

### Branches

The branch pane lists local branches first and remote branches after them, sorted by name inside each group.

- Filled circle: visible local branch.
- Hollow circle: hidden local branch.
- Filled diamond: visible remote branch.
- Hollow diamond: hidden remote branch.

Branch visibility affects graph roots and filtering. Branches are visible by default unless their exact local or remote name is saved in the hidden-branch layer. Hidden branch names are saved per repository, pruned when refs disappear, and new branches are visible until explicitly hidden.

### Tags

The tag pane lists local tags sorted by name. Tags are shown in the graph at the commit they resolve to. The app can create and delete local lightweight tags.

### Stashes

The stash pane lists stash commits. Stashes are real commits and are rendered in the graph near their first parent.

### Reflogs

The reflog pane lists recent HEAD reflog entries. Reflog rows can jump to commits that are visible in the graph. If the reflog commit is hidden, enable graph reflogs with `)` so the walker includes HEAD reflog roots.

### Worktrees

The worktree pane lists the main worktree and linked worktrees.

Rows include:

- Worktree name.
- Branch name, detached HEAD short SHA, or no-head state.
- Current-worktree marker.
- Dirty marker.
- Locked marker.
- Invalid marker.

Valid worktrees can be opened from the pane. Linked worktrees can be locked, unlocked, removed, or pruned through the worktree actions.

### Submodules

The submodule pane lists immediate Git submodules for the current repository.

Rows include:

- Submodule path.
- Current branch, detached HEAD short SHA, or not-initialized state.
- Staged pointer marker.
- New-commits marker.
- Modified-content marker.
- Untracked-content marker.
- Not-open marker.

Initialized submodules can be opened from the pane, which reloads `guitar` at the submodule repository path. Opened submodules are tracked in a session-only stack, rendered in the left status bar as a `▣` breadcrumb before the current branch or HEAD. Uninitialized submodules can be updated and initialized through the submodule action.

### Status

The status area is on the right.

On the uncommitted row:

- The top status pane shows conflicts and staged changes.
- The bottom status pane shows conflicts and unstaged changes.
- Conflicts appear in both panes and are highlighted.

On a commit row:

- The top status pane shows files changed by the selected commit compared with its first parent.
- The bottom status pane is not used.

Status symbols:

- `!` conflict.
- `~` modified.
- `+` added.
- `-` deleted.
- `→` renamed.

### Inspector

The inspector shows selected commit metadata:

- Commit SHA.
- Parent SHAs.
- Featured branch labels.
- Author.
- Committer.
- Summary.
- Body.

When the selected row is uncommitted, the inspector appears if there are conflicts.

### Viewer

The viewer opens from a selected status row. It can show:

- Working tree file contents and diff for the uncommitted row.
- Commit file contents and diff for selected commits.
- Conflict files with conflict-marker highlighting.
- Unified diff style.
- Hunk-only mode.
- Side-by-side split diff mode.
- Line numbers.
- Wrapped long lines.

For merge commits, file lists and file diffs compare against the first parent.

### Settings

The settings/help view is opened with `?`. It shows version and the commit heatmap above tabbed settings sections for general, display, auth, repo, and shortcuts. Recent repository rows, performance rows, remote rows, theme rows, layout rows, and keybinding rows are selectable.

## Navigation

### Scope

`h` and `l` are the most important navigation keys.

- In the graph, `l` narrows into details. On a commit row it focuses the inspector; on the uncommitted row it focuses staged or unstaged status.
- In the inspector, `l` moves to status.
- In status, `l` opens the selected file in the viewer.
- In the viewer, `h` returns to status.
- In status, `h` moves outward to inspector or graph.
- In the graph, `h` opens/focuses the branch pane.
- In branch, tag, stash, reflog, worktree, and submodule panes, `l` jumps into the selected item.
- In settings, `h` returns to the graph.
- In the splash screen, `l` opens the selected recent repository.

### Focus

`Tab` and `Ctrl+n` move to the next focusable pane. `Shift+Tab` and `Ctrl+p` move to the previous focusable pane.

`Ctrl+h`, `Ctrl+j`, `Ctrl+k`, and `Ctrl+l` move focus to the nearest pane in that direction when one is visible. `Ctrl+Left`, `Ctrl+Down`, `Ctrl+Up`, and `Ctrl+Right` are equivalent arrow-key alternatives.

Focusable panes are ordered:

```text
graph/viewer, inspector, staged/commit status, unstaged status, search, submodules, worktrees, reflogs, stashes, tags, branches
```

Hidden panes are skipped. The unstaged status pane is focusable only on the uncommitted row.

### Selection

`Enter` means "open/select" for the current focus:

- Splash: open selected recent repository.
- Settings theme: activate and save theme.
- Settings remote: open the remote action chooser.
- Settings add remote: open the add-remote prompts.
- Settings keybinding: open key capture.
- Settings layout row: toggle that layout option or reset layout.
- Settings graph lane limit row: open a numeric prompt. In normal mode, `-` and `+` shrink or grow the graph lane limit by one.
- Graph: open a worktree badge if the selected commit has valid worktree candidates.
- Branch/tag/stash/reflog panes: jump to the corresponding graph row and center it when possible.
- Worktree pane: open the selected valid worktree.
- Submodule pane: open the selected initialized submodule as the current repository.
- Status panes: open the selected file in the viewer.
- Choice modals: confirm the selected row.

### Back

`Esc` cancels modals or widens back to the graph.

- From graph: opens the splash screen when the history worker is idle.
- From splash: returns to graph when a repository is loaded.
- From viewer/settings/side panes/status/inspector: returns to graph.
- From text-entry modals: clears input and returns to the relevant view.
- From auth prompt: cancels the pending network operation.

### Scrolling

All scroll commands act on the focused pane or viewport:

- `j` / `Down`: one row down.
- `k` / `Up`: one row up.
- `Ctrl+d`: half page down.
- `Ctrl+u`: half page up.
- `PageDown`: page down.
- `PageUp`: page up.
- `Ctrl+Alt+d`: jump halfway toward the end of the focused graph/side pane list.
- `Ctrl+Alt+u`: jump halfway toward the beginning of the focused graph/side pane list.
- `g` / `Home`: beginning.
- `Shift+G` / `End`: end.

In full viewer mode, `Ctrl+d` and `Ctrl+u` jump between diff hunk edges. In hunk and split viewer modes, they scroll by half pages.

### Resizing

`Ctrl+Alt+h`, `Ctrl+Alt+j`, `Ctrl+Alt+k`, and `Ctrl+Alt+l` resize the focused pane one cell or row at a time. `Ctrl+Alt+Left`, `Ctrl+Alt+Down`, `Ctrl+Alt+Up`, and `Ctrl+Alt+Right` are equivalent arrow-key alternatives.

- Left panes: `Ctrl+Alt+l` widens the left column; `Ctrl+Alt+h` narrows it.
- Right panes: `Ctrl+Alt+h` widens the right column; `Ctrl+Alt+l` narrows it.
- Graph/viewer focus: `Ctrl+Alt+h` grows the center left by shrinking the left pane; `Ctrl+Alt+l` grows the center right by shrinking the right pane.
- Split diff viewer: horizontal resizing still resizes the outer viewer area; the split divider stays centered.
- Stacked panes: `Ctrl+Alt+k` grows upward when possible; `Ctrl+Alt+j` grows downward when possible.

Keyboard resize changes are saved to `layout.json`. Keyboard resize is disabled in splash, settings, modals, and zen mode.

### Graph Jumps

- `{`: jump to the previous visible branch-bearing commit.
- `}`: jump to the next visible branch-bearing commit.
- `[`: jump to the first loaded child of the selected commit.
- `]`: jump to the first parent of the selected commit.
- `/`: open SHA-prefix lookup for loaded history.

SHA lookup accepts non-empty prefixes up to 40 characters. It searches commits already known to the worker.

### Mouse

Mouse capture is enabled while the app runs.

- Mouse wheel scrolls the pane under the cursor and focuses it.
- Click or drag a visible scrollbar track to jump or scrub the pane it belongs to.
- A single click selects the row under the cursor and focuses that pane.
- Double-click branch, tag, stash, reflog, worktree, submodule, status, theme, or keybinding rows to perform the same action as `Enter`.
- Single-click settings layout rows to toggle them immediately.
- Graph, viewer, and splash rows are selected by mouse clicks but not activated by double-click.
- Drag the left and right vertical dividers to resize side panes.
- Drag stacked pane dividers to resize branch/tag/stash/reflog/worktree/submodule, inspector/status, and staged/unstaged splits.
- Layout changes from dragging are saved when the mouse button is released.

## File Search

`Shift+F` opens repository file search from the graph, inspector, status panes, search pane, or side panes.

Type part of a tracked filepath to filter suggestions. Matching characters are highlighted in the suggestion list. Use `Up`, `Down`, `Ctrl+k`, or `Ctrl+j` to move through suggestions, `Enter` to choose the selected path, and `Esc` to cancel.

Choosing a file opens the search pane and loads commits that changed that path. The search pane shows each matching commit with a status marker, short SHA, and commit summary. Selecting a search result and pressing `Enter` jumps the graph to that commit.

While the search pane is open, matching commits are highlighted in the graph. If the searched path also appears in the staged, unstaged, or conflict status panes, that filepath row is highlighted there too.

Use `7` to show or hide the search pane.

## Inputs And Keymaps

### Input Modes

There are two keymap modes:

- Normal mode: navigation, layout, safe operations, and non-destructive creation prompts.
- Action mode: single-shot mode for dangerous or destructive operations.

By default, `Ctrl+a` enters action mode. The next handled key runs through the action keymap and then the mode returns to normal.

Action mode inherits normal-mode navigation and safe operations, but overrides or adds the dangerous bindings listed below. Notably, action-mode `r` is rebase, action-mode `Shift+R` is revert, and action-mode `m` is merge. The graph lane limit `-` and `+` shortcuts are normal-mode only.

### Text Inputs

Text prompts are single-line inputs.

Supported editing keys:

- Printable character keys insert text.
- `Backspace` removes before the cursor.
- `Delete` removes at the cursor.
- `Left` and `Right` move the cursor.
- `Home` and `End` jump to the start or end.
- `Enter` submits when the prompt accepts the current value.
- `Esc` cancels.

Text prompts are used for commit messages, cherry-pick messages, revert messages, branch names, tag names, worktree names, worktree paths, worktree lock reasons, SHA lookup, and auth fields.

### Auth Inputs

The auth modal uses:

- `Tab`, `Shift+Tab`, `Up`, or `Down` to switch between username and secret fields for HTTP/HTTPS prompts.
- `Enter` to submit.
- `Esc` to cancel the network operation.

For SSH passphrase prompts, focus stays on the secret field.

### Key Capture

In settings, selecting a keybinding opens key capture.

- Press a new key combination to preview it.
- `Enter` confirms if there is no conflict.
- `Esc` or `Ctrl+C` cancels.

If a normal-mode binding is also present in action mode for the same command, rebinding the normal key syncs the matching action-mode binding.

### Default Normal Mode Keymap

Defaults are written to `keymap.json` on first run. User-edited keymaps can differ.

| Command | Default keys |
| --- | --- |
| Widen Scope | `h`, `Left` |
| Narrow Scope | `l`, `Right` |
| Select | `Enter` |
| Back | `Esc` |
| Focus Previous Pane | `Ctrl+p`, `Shift+BackTab` |
| Focus Next Pane | `Ctrl+n`, `Tab` |
| Focus Pane Left | `Ctrl+h`, `Ctrl+Left` |
| Focus Pane Down | `Ctrl+j`, `Ctrl+Down` |
| Focus Pane Up | `Ctrl+k`, `Ctrl+Up` |
| Focus Pane Right | `Ctrl+l`, `Ctrl+Right` |
| Resize Pane Left | `Ctrl+Alt+h`, `Ctrl+Alt+Left` |
| Resize Pane Down | `Ctrl+Alt+j`, `Ctrl+Alt+Down` |
| Resize Pane Up | `Ctrl+Alt+k`, `Ctrl+Alt+Up` |
| Resize Pane Right | `Ctrl+Alt+l`, `Ctrl+Alt+Right` |
| Scroll Down | `j`, `Down` |
| Scroll Up | `k`, `Up` |
| Scroll Down Half | `Ctrl+Alt+d` |
| Scroll Up Half | `Ctrl+Alt+u` |
| Scroll Half Page Down | `Ctrl+d` |
| Scroll Half Page Up | `Ctrl+u` |
| Scroll Page Up | `PageUp` |
| Scroll Page Down | `PageDown` |
| Go To Beginning | `g`, `Home` |
| Go To End | `Shift+G`, `End` |
| Find | `/` |
| Find File | `Shift+F` |
| Scroll Up Branch | `{` |
| Scroll Down Branch | `}` |
| Scroll Up Commit | `[` |
| Scroll Down Commit | `]` |
| Toggle Hunk Mode | `m` |
| Toggle Split Diff Mode | `v` |
| Toggle Zen Mode | `z` |
| Reset Layout | `0` |
| Toggle Branches | `1` |
| Toggle Tags | `2` |
| Toggle Stashes | `3` |
| Toggle Reflogs | `4` |
| Toggle Worktrees | `5` |
| Toggle Submodules | `6` |
| Toggle Search | `7` |
| Toggle Inspector | `8` |
| Toggle Status | `9` |
| Toggle Graph Reflogs | `)` |
| Toggle SHAs | `!` |
| Toggle Graph Committer Date/Time | `@` |
| Toggle Graph Committers | `#` |
| Toggle Graph Refs | `$` |
| Shrink Graph Lane Limit | `-` |
| Grow Graph Lane Limit | `+` |
| Toggle Help / Settings | `?` |
| Return To Parent Repository | `Backspace` |
| Action Mode | `Ctrl+a` |
| Minimize | `.` |
| Reload | `r` |
| Reload and Show All Branches | `Shift+R` |
| Exit | `q` |
| Remove Recent Repository | `d` |
| Move Recent Repository Up | `Shift+K` |
| Move Recent Repository Down | `Shift+J` |
| Stage | `s` |
| Unstage | `u` |
| Commit | `c` |
| Fetch All | `f` |
| Create Branch | `b` |
| Create Worktree | `w` |
| Tag | `t` |
| Toggle Selected Branch | `Shift+T` |
| Solo Selected Branch | `Space` |

### Default Action Mode Keymap

Press `Ctrl+a`, then one of these keys.

| Command | Default key after `Ctrl+a` |
| --- | --- |
| Drop Stash | `x` |
| Pop Stash | `p` |
| Stash Worktree | `Shift+S` |
| Checkout | `o` |
| Hard Reset | `Shift+H` |
| Mixed Reset | `Shift+M` |
| Force Push | `Shift+P` |
| Push Tags | `Shift+V` |
| Delete Branch | `Shift+D` |
| Rename Branch | `Shift+B` |
| Remove Worktree | `Shift+W` |
| Toggle Worktree Lock | `Shift+L` |
| Update Submodule | `i` |
| Sync Submodule | `Shift+I` |
| Delete Tag | `Shift+U` |
| Cherry-pick | `y` |
| Revert | `Shift+R` |
| Rebase | `r` |
| Merge | `m` |
| Continue Rebase/Cherry-pick/Revert/Merge | `Shift+C` |
| Abort Rebase/Cherry-pick/Revert/Merge | `Shift+A` |

## Git Operations

Operations depend on the focused pane and selected row. Most operations call `reload` on success so the graph, status, panes, and recent path state refresh.

### Stage

Normal key: `s`.

- Graph focus on the uncommitted row stages all unstaged changes, including untracked files and deletes.
- Status bottom focus stages the selected unstaged file.
- Submodule pane focus stages the selected submodule's checked-out `HEAD` as the superproject gitlink pointer.
- Conflict rows cannot be staged from the UI. Resolve conflicts externally, then continue the active operation.

Ignored files are not staged.

### Unstage

Normal key: `u`.

- Graph focus on the uncommitted row unstages all staged files with a mixed reset to `HEAD`.
- Status top focus unstages the selected staged file.
- Submodule pane focus unstages the selected submodule gitlink pointer in the superproject index.
- In a repository without `HEAD`, unstaging a file removes it from the initial index.
- Conflict rows cannot be unstaged from the UI.

### Commit

Normal key: `c`.

`Commit` opens a single-line commit message prompt when staged changes exist. The commit uses the configured `user.name` and `user.email`. If the branch is unborn, the commit becomes the root commit.

### Fetch

Normal key: `f`.

Fetch runs as a background network operation against the repository's default remote. The default remote is resolved from `guitar.defaultRemote`, `remote.pushDefault`, the current branch upstream, `origin`, then the first configured remote.

It fetches:

- `refs/heads/*` into `refs/remotes/<default>/*`.
- `refs/tags/*` into local tags.

Pruning is enabled.

### Remotes

Remote management lives in settings. Open settings with `?`, select a remote row, and press `Enter`.

Remote rows can:

- Fetch the selected remote with the same background auth/progress flow as normal fetch.
- Set the selected remote as the default for normal fetch, force-push, and push-tags.
- Rename the remote.
- Edit the fetch URL.
- Edit the push URL.
- Delete the local remote configuration.

The `+ add remote` row opens prompts for a remote name and fetch URL. Remote names are validated with Git's remote-name rules.

The default remote is saved in repo-local Git config as `guitar.defaultRemote` and mirrored to `remote.pushDefault`. Editing a push URL to an empty value clears the dedicated push URL, so Git falls back to the fetch URL. Deleting a remote removes local remote configuration and prunes matching hidden-branch visibility entries; it does not delete anything on the server.

Normal-key `f`, force-push current branch, and push-tags target the default remote. Fetching from a selected remote row still targets that selected remote.

### Checkout

Action key: `Ctrl+a`, then `o`.

Checkout is forceful and uses libgit2 checkout options with conflicts allowed.

- Branch pane focus checks out the selected branch.
- If the selected branch is local, it is checked out directly.
- If the selected branch is remote, `guitar` materializes a local branch using the remote branch's short name, sets upstream to the remote branch, and checks it out.
- Graph focus checks out the selected commit.
- If the commit has no branch label, graph checkout uses detached HEAD.
- If the commit has one branch label, that branch is checked out.
- If the commit has multiple branch labels, a checkout modal lets you choose.

The uncommitted row cannot be checked out.

### Branch Visibility

Normal keys:

- `Shift+T`: toggle selected branch visibility.
- `Space`: solo selected branch.

Branch visibility filters graph roots and branch labels.

- From branch pane focus, the action applies to the selected branch.
- From graph focus, the action applies to branch labels on the selected commit.
- If multiple branch labels are present, a modal lets you choose.
- Toggling a visible branch adds it to the hidden layer.
- Toggling a hidden branch removes it from the hidden layer.
- Solo hides every current branch except the selected branch.
- If filtering would hide every current branch, the hidden layer resets to all visible.

### Create Branch

Normal key: `b`.

- Graph focus creates a local branch at the selected commit.
- Reflog focus creates a local branch at the selected reflog entry's new commit.
- Branch creation does not check out the new branch.
- The selected uncommitted row is not a branch target.

### Delete Branch

Action key: `Ctrl+a`, then `Shift+D`.

- Branch pane focus deletes the selected branch.
- Graph focus deletes a branch label attached to the selected commit.
- If multiple deletable labels exist, a modal lets you choose.
- The current branch cannot be deleted.
- Local branch deletion removes the local branch ref.
- Remote branch deletion pushes an empty source refspec to the selected remote branch.

Remote deletion derives the remote from the branch name. For example, `origin/topic` deletes `topic` from `origin`.

### Rename Branch

Action key: `Ctrl+a`, then `Shift+B`.

- Branch pane focus renames the selected local branch.
- Graph focus renames a local branch label attached to the selected commit.
- If multiple local branch labels are present, a modal lets you choose.
- Remote branches cannot be renamed from `guitar`.
- Rename opens a single-line branch name prompt. Invalid, unchanged, or existing names show the error modal and return to the prompt.
- Renaming a branch does not push, delete, or rename any remote branch.

### Tags

Create tag: normal key `t`.

Delete tag: action key `Ctrl+a`, then `Shift+U`.

- Tag creation creates a lightweight local tag at the selected commit.
- If the graph is on the uncommitted row, tag creation targets the first real commit.
- Tag deletion from the tag pane deletes the selected local tag.
- Tag deletion from the graph deletes a tag attached to the selected commit. If multiple tags exist, a modal lets you choose.
- `guitar` can push local tags, but it does not delete remote tags.

### Push

Force push current branch: action key `Ctrl+a`, then `Shift+P`.

Push tags: action key `Ctrl+a`, then `Shift+V`.

- Branch push targets the default remote.
- Branch push is force push only and updates the current local branch on the remote branch with the same name.
- Detached HEAD cannot be pushed.
- Tag push pushes all local tags to the default remote.
- If no local tags exist, push-tags succeeds without changing anything.

### Stash

Stash current work: action key `Ctrl+a`, then `Shift+S`.

Pop selected stash: action key `Ctrl+a`, then `p`.

Drop selected stash: action key `Ctrl+a`, then `x`.

- Stash includes untracked files.
- Stash messages are generated from the current `HEAD` short SHA and summary.
- Pop applies the stash and drops it.
- Drop removes the stash without applying it.
- Pop/drop operate only when graph focus is on a stash row.

### Reset

Hard reset selected commit: action key `Ctrl+a`, then `Shift+H`.

Mixed reset selected commit: action key `Ctrl+a`, then `Shift+M`.

Discard selected status file: focus a status row and use action key `Ctrl+a`, then `Shift+H`.

- Graph hard reset moves the current branch or detached `HEAD` to the selected commit and rewrites index and working tree.
- Graph mixed reset moves the current branch or detached `HEAD` to the selected commit and rewrites the index while leaving working tree contents.
- File hard reset removes staged and working tree changes for the selected path by restoring it from `HEAD`.

### Cherry-pick

Action key: `Ctrl+a`, then `y`.

- Graph focus on a commit opens a single-line message prompt.
- The default message is `cherrypicked: <selected summary>`.
- The working tree must be clean before starting.
- If there are no conflicts, `guitar` commits immediately with the provided message.
- If conflicts occur, `guitar` stops and shows a conflict modal.
- Resolve files externally, then continue with `Ctrl+a`, `Shift+C`.
- Abort with `Ctrl+a`, `Shift+A`.

During an in-progress conflicted cherry-pick, the message is stored at `.git/GUITAR_CHERRYPICK_MSG` and removed on commit or abort.

### Revert

Action key: `Ctrl+a`, then `Shift+R`.

- Graph focus on a commit opens a single-line message prompt.
- The default message is `reverted: <selected summary>`.
- Merge commits cannot be reverted in this version.
- The working tree must be clean before starting.
- If there are no conflicts, `guitar` commits immediately with the provided message.
- If conflicts occur, `guitar` stops and shows a conflict modal.
- Resolve files externally, then continue with `Ctrl+a`, `Shift+C`.
- Abort with `Ctrl+a`, `Shift+A`.

During an in-progress conflicted revert, the message is stored at `.git/GUITAR_REVERT_MSG` and removed on commit or abort.

### Rebase

Action key: `Ctrl+a`, then `r`.

- Rebases the current local branch onto the selected graph commit.
- Detached HEAD cannot be rebased.
- The selected commit cannot already be `HEAD`.
- The working tree must be clean before starting.
- Commits are driven through libgit2's rebase API.
- If conflicts occur, resolve files externally, then continue with `Ctrl+a`, `Shift+C`.
- Abort with `Ctrl+a`, `Shift+A`.

If a rebase, cherry-pick, revert, or merge is already in progress, pressing the rebase action attempts to continue the active operation.

### Merge

Action key: `Ctrl+a`, then `m`.

- Merges the selected graph commit into the current local branch.
- Detached HEAD cannot be merged into.
- Another in-progress Git operation blocks merge start.
- The working tree must be clean before starting.
- Merge analysis honors Git's fast-forward preferences, including `merge.ff=only`.
- Fast-forward, up-to-date, normal merge commit, conflict, and abort states are surfaced in the UI.
- Continue with `Ctrl+a`, `Shift+C`.
- Abort with `Ctrl+a`, `Shift+A`.

If a rebase, cherry-pick, revert, or merge is already in progress, pressing the merge action attempts to continue the active operation.

### Continue And Abort

Continue active operation: action key `Ctrl+a`, then `Shift+C`.

Abort active operation: action key `Ctrl+a`, then `Shift+A`.

These apply to active rebase, cherry-pick, revert, or merge states.

On continue, `guitar` checks the index for conflicts. For conflicted paths, it inspects the worktree file:

- If the file still contains conflict markers, it remains conflicted.
- If the file exists and conflict markers are gone, it is added to the index.
- If the file was removed, it is removed from the index.

Then the active Git operation continues.

### Worktrees

Create worktree: normal key `w`.

Remove worktree: action key `Ctrl+a`, then `Shift+W`.

Lock or unlock worktree: action key `Ctrl+a`, then `Shift+L`.

Open worktree: focus a worktree row and press `Enter`; or press `Enter` on a graph worktree badge.

Create worktree flow:

1. Focus the graph on a real commit.
2. Press `w`.
3. Enter a worktree name.
4. Confirm or edit the default path.

The worktree name becomes a new local branch name. Names cannot be empty, `.`, `..`, or contain path separators. If worktree creation fails after branch creation, `guitar` attempts to delete the branch it created.

Default worktree paths are based on the current repository path:

```text
<repo-parent>/<repo-name>-<worktree-name>
```

Removal rules:

- Only linked worktrees can be removed.
- The current worktree cannot be removed.
- The main worktree cannot be removed.
- Locked worktrees cannot be removed.
- Valid worktrees are pruned with working-tree removal enabled.

Locking rules:

- Only valid linked worktrees can be locked.
- Locking opens a reason prompt. Empty reasons are allowed and become no reason.
- Locked linked worktrees can be unlocked with the same action.

Opening a worktree reloads the app at the selected worktree path.

### Submodules

Toggle the submodule pane: normal key `6`.

Update or initialize selected submodule: action key `Ctrl+a`, then `i`.

Sync selected submodule URL: action key `Ctrl+a`, then `Shift+I`.

Open submodule: focus an initialized submodule row and press `Enter`.

Return to parent repository: normal key `Backspace`.

Stage selected submodule pointer: focus a submodule row and press `s`.

Unstage selected submodule pointer: focus a submodule row and press `u`.

Submodule support is intentionally bounded:

- Only immediate submodules of the currently opened repository are listed.
- Opening a submodule reloads `guitar` at that submodule path and pushes the parent onto a session-only return stack.
- The stack is not persisted and is not inferred when launching directly inside a submodule.
- Update/init uses the same background network worker and auth prompts as fetch/push operations.
- Sync mirrors Git's submodule URL sync behavior for the selected submodule.
- Add, remove, edit-url, and recursive update flows are not implemented.
- Commit and status file diffs stay file-oriented and do not recurse into submodule commit graphs.

## Authentication

Network auth is used for fetch, selected-remote fetch from settings, push current branch, push tags, remote branch deletion, and submodule update/init.

### SSH

For SSH remotes, `guitar` tries:

1. Username from the URL or callback, falling back to `git`.
2. `ssh-agent` via libgit2.
3. A default private key without passphrase.
4. An in-app passphrase prompt if a default private key exists.

Default private key search order:

```text
~/.ssh/id_ed25519
~/.ssh/id_ecdsa
~/.ssh/id_rsa
```

If no `ssh-agent` credential works and none of those keys exist, the operation fails with an SSH auth error.

### HTTP And HTTPS

For HTTP/HTTPS remotes, `guitar` tries:

1. Any matching secret already entered during the current `guitar` session.
2. Git credential helper through libgit2.
3. Username callback if requested by libgit2.
4. In-app username/password or username/token prompt.

Use a personal access token as the password when your hosting provider requires token auth.

### Auth Attempts And Secret Storage

- A network operation allows up to 3 auth prompt attempts.
- Rejected in-memory secrets are evicted before retry.
- Prompted secrets are stored only in memory for the running process.
- No prompted SSH passphrases, HTTPS passwords, or HTTPS tokens are written to `guitar` config files.
- Existing Git credential helpers may store credentials according to your Git configuration; that storage is outside `guitar`.

Only one network operation can run at a time.

## Settings

Open settings with `?`.

The settings view includes app version and commit heatmap above these tabs:

- `general`: config file paths, performance settings, and recent repositories.
- `display`: pane visibility, graph metadata toggles, language, symbol themes, and theme list.
- `auth`: Git `user.name`, `user.email`, and auth behavior notes.
- `repo`: remotes and remote URLs.
- `shortcuts`: normal-mode shortcuts and action-mode shortcuts that differ from normal mode.

Selectable rows:

- Recent repository rows: `d` removes, `Shift+K` moves up, and `Shift+J` moves down.
- Remote rows: `Enter` opens fetch, set-default, rename, URL edit, push URL edit, and delete actions.
- Add remote row: `Enter` opens name and URL prompts.
- Theme rows: `Enter` activates and saves the selected theme.
- Display toggle rows: `Enter` toggles the row or resets layout.
- Graph lane limit row: `Enter` opens a numeric prompt. Positive values save to `layout.json`; `0` and invalid input keep the modal open without changing the setting. In normal mode, `-` and `+` shrink or grow the saved graph lane limit by one and reload an open repository.
- Keybinding rows: `Enter` opens key capture.

Settings reuses normal navigation. Use `Tab` / `Shift+Tab` or click a tab label to switch tabs. Use `j`/`k`, page keys, `g`, `Shift+G`, or mouse wheel to move within the active tab. Use `h`, `Esc`, or `?` to return to the graph.

Available built-in themes:

<img width="100%" src="https://github.com/user-attachments/assets/95b5d056-2460-4435-9d26-b40ee20538fb" />

```text
classic
ansi
monochrome
dracula dark
dracula light
monokai dark
monokai light
catppuccin dark
catppuccin light
atom dark
atom light
vscode dark
vscode light
solarized dark
solarized light
gruvbox dark
gruvbox light
nord
tokyo night
tokyo night storm
tokyo night light
github dark
github light
github dark dimmed
night owl
light owl
ayu dark
ayu mirage
ayu light
material
palenight
rose pine
rose pine moon
rose pine dawn
kanagawa wave
kanagawa dragon
kanagawa lotus
everforest dark
everforest light
zenburn
horizon
synthwave 84
base16 tomorrow
base16 ocean
base16 eighties
matrix
```

## Persistence

Saved app files live under your platform config directory in a `guitar` folder.

Common examples:

```text
Linux:   ~/.config/guitar
macOS:   ~/Library/Application Support/guitar
Windows: %APPDATA%\guitar
```

The app writes:

- `keymap.json`: keyboard mappings.
- `layout.json`: pane visibility, widths, weights, graph metadata display, graph reflog setting, graph lane limit, zen/minimal state.
- `theme.json`: active theme and all color slots.
- `symbols.json`: active symbol theme and all configurable UI symbols.
- `recent.json`: recent repository paths.
- `branch_visibility.json`: per-repository hidden branch names.

The app may also temporarily write `.git/GUITAR_CHERRYPICK_MSG` inside a repository during a conflicted cherry-pick and `.git/GUITAR_REVERT_MSG` during a conflicted revert.

Not persisted by `guitar`:

- SSH key passphrases entered in the auth modal.
- HTTPS usernames, passwords, or tokens entered in the auth modal.
- Current selection, scroll positions, open modal state, or current viewport.

Reset all saved app config:

```bash
guitar --reset
```

## Configuration Files

### keymap.json

`keymap.json` has `normal` and `action` arrays.

Example entry:

```json
{
  "key": "Char(j)",
  "modifiers": [],
  "command": "ScrollDown"
}
```

Supported key strings include:

```text
Backspace
Enter
Left
Right
Up
Down
Home
End
PageUp
PageDown
Tab
BackTab
Delete
Insert
Null
Esc
CapsLock
ScrollLock
NumLock
PrintScreen
Pause
F(n)
Char(x)
```

Supported modifiers:

```text
Shift
Control
Ctrl
Alt
Command
Meta
```

Supported commands are the command names listed in the keymap tables, without spaces, for example `ToggleSplitDiffMode`, `FocusPaneRight`, `ResizePaneRight`, `RemoveRecentRepository`, `CreateWorktree`, `ToggleSubmodules`, `ToggleGraphDates`, `ToggleGraphCommitters`, `ToggleGraphRefs`, `ShrinkGraphLaneLimit`, `GrowGraphLaneLimit`, `ReturnToParentRepository`, `UpdateSubmodule`, `SyncSubmodule`, `Revert`, `ContinueOperation`, and `AbortOperation`.

Existing `keymap.json` files are preserved. When a new default command is missing and its default key is unbound, `guitar` may add that binding automatically; otherwise, edit the keymap or reset saved config to adopt changed defaults.

If an existing keymap still has the old untouched number/symbol pane-toggle defaults, `guitar` migrates that row to the current `1`-through-`9` pane toggles and graph metadata toggles on `)` through `$`. If it still has the untouched `Ctrl+0`-through-`Ctrl+4` graph metadata defaults from an earlier version, `guitar` migrates those to the same graph metadata commands.

Terminals vary in how they report shifted number keys. `guitar` stores these graph metadata keys as Shift-modified digits but displays them as `)`, `!`, `@`, `#`, and `$`, and accepts those shifted-character events as fallbacks. `Ctrl+digit` remains terminal-dependent, but when a terminal emits distinct `Ctrl+0` through `Ctrl+4` events they fall back to the same graph metadata commands.

### layout.json

Default layout:

```json
{
  "is_shas": true,
  "is_minimal": false,
  "is_branches": true,
  "is_tags": false,
  "is_stashes": false,
  "is_reflogs": false,
  "is_graph_reflogs": false,
  "is_graph_dates": false,
  "is_graph_committers": false,
  "is_graph_refs": true,
  "is_worktrees": false,
  "is_submodules": false,
  "is_status": true,
  "is_inspector": true,
  "is_zen": false,
  "width_left_pane": 45,
  "width_right_pane": 46,
  "weight_branches": 100,
  "weight_tags": 100,
  "weight_stashes": 100,
  "weight_reflogs": 100,
  "weight_worktrees": 100,
  "weight_submodules": 100,
  "weight_inspector": 100,
  "weight_status": 100,
  "weight_status_top": 100,
  "weight_status_bottom": 100,
  "weight_viewer_split_left": 100,
  "weight_viewer_split_right": 100,
  "graph_lane_limit": 50
}
```

Width values are clamped to a minimum of 16 columns. The center pane keeps at least 20 columns when possible. Stacked pane weights are normalized to at least `1`. Split viewer weights may exist in older configs, but the split divider is always centered.

`graph_lane_limit` defaults to `50` and is normalized to at least `1`.

The graph reflog toggle requires a reload because it changes graph roots.

### theme.json

`theme.json` stores a label and full color map. Loading a known preset label without custom colors resolves to that preset. Unknown labels or color overrides become a custom theme.

Example:

```json
{
  "label": "solarized dark",
  "colors": {
    "red": "#dc322f",
    "green": "#859900",
    "blue": "#268bd2",
    "grey_950": "#002b36",
    "border": "#073642",
    "text": "#839496",
    "highlighted": "#93a1a1"
  }
}
```

Color values can be:

- Hex colors like `#268bd2`.
- Terminal color names such as `red`, `green`, `blue`, `gray`, `dark_gray`, `light_red`, `white`, or `reset`.
- Indexed terminal colors like `indexed:42` or `indexed_42`.

Color slots:

```text
red, pink, purple, durple, indigo, blue, cyan, teal, green, grass,
lime, yellow, amber, orange, grapefruit, brown, dark_red,
light_green_900, grey_50, grey_100, grey_200, grey_300, grey_400,
grey_500, grey_600, grey_700, grey_800, grey_900, grey_950,
border, text, highlighted
```

Malformed theme files fall back to `classic` and are rewritten.

### symbols.json

`symbols.json` stores the active symbol theme label and a full symbol map. The Display settings tab can switch between the built-in `main` and `ascii` symbol themes.

- `main` matches the default Unicode and Nerd Font symbols.
- `ascii` uses strict ASCII for graph marks, borders, forms, heatmap cells, status symbols, splash markers, and widget chrome.

Users can customize individual symbols by editing `symbols.json`. Loading starts from the label's preset, applies any provided symbol overrides, then rewrites a normalized full file. Unknown labels or overrides become a custom symbol theme.

Example:

```json
{
  "label": "ascii",
  "symbols": {
    "branch": {
      "local_visible": "*",
      "local_hidden": "o"
    },
    "border": {
      "horizontal": "-",
      "vertical": "|",
      "rounded_top_left": "+"
    },
    "form": {
      "checkbox_off": "[ ]",
      "checkbox_on": "[x]"
    }
  }
}
```

Malformed symbol files fall back to `main` and are rewritten.

### recent.json

`recent.json` is a JSON array of absolute repository root paths:

```json
[
  "/home/me/src/project-a",
  "/home/me/src/project-b"
]
```

Recent repositories are appended when a repository opens successfully. They can be removed or reordered from the splash screen or the settings recent repositories section.

## Development

Build:

```bash
cargo build
cargo build --release
```

Run from source:

```bash
cargo run -- .
cargo run -- ../path/to/repository
```

Test:

```bash
cargo test
```

Format:

```bash
cargo fmt
```

The value printed by `guitar --version` comes from Cargo package metadata. Release builds can override it at compile time with `GUITAR_BUILD_OVERWRITE_VERSION`.

Important source areas:

- `src/main.rs`: CLI flags and app startup.
- `src/app/app.rs`: main app state, event loop, draw orchestration, reload, graph worker sync.
- `src/app/input/`: keyboard, mouse, modal, navigation, Git, worktree, and submodule input handlers.
- `src/app/draw/`: TUI drawing for graph, panes, viewer, settings, status, and modals.
- `src/core/`: graph worker, walker, topology buffer, pane data, render helpers.
- `src/git/actions/`: mutating Git operations.
- `src/git/queries/`: repository reads, diffs, commits, reflogs, worktrees, submodules.
- `src/git/auth.rs`: network credential classification, prompting, and session cache.
- `src/helpers/`: keymaps, layout persistence, themes, recent repos, symbols, text, colors.

## Known Limitations

- No filesystem watcher. Use reload when repository state changes outside the app.
- Current branch push is force push only.
- No pull UI.
- Conflict resolution editing is external.
- Worktree move/repair and custom separate worktree branch names are not implemented.
- Submodule support covers immediate submodules only; commit and status file diffs do not recurse into submodule commit graphs.
- Merge commit file lists and file diffs compare against the first parent only.
- The graph renderer models ordinary two-parent merges; octopus merges are not a first-class display target.
- Tag creation is lightweight only.
- No annotated-tag message flow.
- No remote tag deletion flow.
- Branch upstream editing is not implemented.
- Search does not match commit messages, authors, branches, tags, filenames, or unloaded history.

## Roadmap

Planned or desired features include jujutsu integration, richer worktree management, richer in-app conflict resolution, and more.

Follow the project board for current work:

[https://github.com/users/asinglebit/projects/1/views/1](https://github.com/users/asinglebit/projects/1/views/1)

## Screenshots

![screenshot 1](https://github.com/user-attachments/assets/37df457b-bbf4-4d51-a965-c300b426cb62)
![screenshot 2](https://github.com/user-attachments/assets/b09f73e7-5bda-4ecd-ba7c-3bba85395e37)
![screenshot 3](https://github.com/user-attachments/assets/2ed8f14e-193b-4815-b37e-283bd129787f)
![screenshot 4](https://github.com/user-attachments/assets/15e4630f-a141-4724-9d35-1b8601006598)
![screenshot 5](https://github.com/user-attachments/assets/10389ec5-6780-4bcb-85dc-67f9e012ed63)
![screenshot 6](https://github.com/user-attachments/assets/a408af0c-ef85-4692-914b-81562d3873e4)
![screenshot 7](https://github.com/user-attachments/assets/72d09d11-86cb-4356-a3dd-93684abc9b19)
