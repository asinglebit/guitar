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

![2](https://github.com/user-attachments/assets/177dbf13-b9ad-480e-a1be-71a333454a44)

`guita╭` is a Rust terminal UI for working with Git history from a topology-first point of view. It is built with `ratatui` and `libgit2`, and aims to make large repositories navigable without leaving the terminal.

### Demo

Here is an older recording of the v0.1.12 feature set:

[https://www.youtube.com/watch?v=oERA8MYlHjQ](https://www.youtube.com/watch?v=oERA8MYlHjQ)

The recording is still useful as a high-level tour, but the current README and in-app settings/help screen are the source of truth for current shortcuts and behavior.

### Disclaimer

This is a hobby project, and it includes sharp tools. Some actions are intentionally destructive, and several workflows still favor the happy path over a fully guided Git client experience. Use it with caution on important repositories, keep backups, and please report issues or contribute fixes if something feels wrong.

### Motivation

I wanted a terminal-based, cross-platform Git client that makes it easy to understand where I am topologically at any point in time. It needed to be fast, useful in day-to-day work, and a good reason to learn Rust. This project is where those goals met.

### Requirements

- A Rust toolchain with Cargo and Rust support.
- `user.name` and `user.email` configured in Git before launching a repository. The app reads them on repository load and uses them for commits.
- An external `ssh-agent` for network operations such as fetch, force push, tag push, and remote branch deletion.

### Install

Prebuilt binaries are published on the releases page:

[https://github.com/asinglebit/guitar/releases](https://github.com/asinglebit/guitar/releases)

To build from source:

```bash
git clone https://github.com/asinglebit/guitar.git
cd guitar
cargo build --release
```

The release binary will be at:

```bash
target/release/guitar
```

### Usage

Run `guitar` from inside a repository, from a repository subdirectory, or with an explicit path:

```bash
guitar
guitar ../path/to/your/repo
```

If the path cannot be opened as a Git repository, the app falls back to the splash screen and shows recent repositories if any are saved.

Useful flags:

```bash
guitar --version
guitar -v
guitar --reset
```

`--reset` deletes the saved `guitar` config directory so layout, keymap, and recent repository files can be regenerated.

### Current Features

#### Repository Graph

- Incremental history loading in a background walker. Large repositories become usable while history continues loading in batches.
- Graph rendering for commits, branches, merges, tags, stashes, and the synthetic uncommitted-work row.
- Optional abbreviated SHA column.
- Branch visibility filters and one-branch solo mode.
- Navigation by row, page, half page, list midpoint, first/last row, branch labels, and first-parent commit relationships.
- SHA-prefix jump for commits that have already been loaded.
- Recent repository splash screen.
- Status bar with current branch, detached HEAD, unborn repository state, selection counts, loading spinner, action-mode indicator, and zen-mode indicator.

#### Panes and Views

- Branch pane with local/remote and visible/hidden indicators.
- Tag pane for local tags.
- Stash pane showing stash commits alongside normal history.
- Status panes that split staged and unstaged files on the uncommitted row.
- Commit file list for the selected commit, compared with its first parent.
- Commit inspector with commit SHA, parent SHAs, featured branches, author, committer, summary, and body.
- File viewer for selected status/commit files, with line wrapping, line numbers, diff highlighting, hunk-only mode, and split diff mode.
- Settings/help view with version, commit heatmap, config paths, Git identity, auth notes, theme selection, and active keymaps.
- Zen mode for focusing one pane at a time.
- Minimal chrome mode that hides the title and status bars.

#### Git Operations

- Stage all unstaged files from the graph row, or stage one selected unstaged file from the status pane.
- Unstage all staged files from the graph row, or unstage one selected staged file from the status pane.
- Commit staged changes with an in-app commit message prompt.
- Fetch `origin` heads and tags over SSH, with pruning enabled.
- Force push the current branch to `origin` over SSH.
- Push all local tags to `origin` over SSH.
- Checkout local branches, materialize and checkout remote branches, or checkout an unlabeled commit in detached HEAD mode.
- Create a branch at the selected commit.
- Delete a local branch, or delete a remote branch over SSH when the selected branch is remote.
- Create and delete lightweight local tags.
- Stash current work, including untracked files.
- Pop or drop a selected stash.
- Hard reset or mixed reset to the selected commit.
- Discard changes for a selected status file by resetting it to `HEAD`.
- Cherry-pick a selected commit in a basic happy-path flow.

#### Input, Layout, and Persistence

- Vim-like default navigation with a customizable keymap.
- Single-shot action mode for dangerous operations. Press the action-mode key first, then the action key.
- Mouse wheel scrolling over panes.
- Mouse dragging for side-pane widths, stacked-pane heights, and the split diff divider.
- Persistent layout toggles, pane widths, pane weights, and split diff divider position.
- Persistent recent repository list.
- Three built-in themes: classic, ANSI, and monochrome.

Saved files live under your platform config directory in a `guitar` folder, for example `~/.config/guitar` on many Linux systems. The app currently writes:

- `keymap.json`
- `layout.json`
- `recent.json`

### Known Limitations and Missing Features

- There is no filesystem watcher. Use reload when repository state changes outside the app.
- Network operations assume the `origin` remote and SSH-agent auth. There is no remote picker, HTTPS credential prompt, or in-app credential flow.
- There is no normal non-force branch push command yet. The current branch push command is force push only.
- There is no pull UI.
- Merge, rebase, and conflict-resolution workflows are not implemented.
- Cherry-pick is still rough: it auto-commits, has no conflict-resolution UI, and does not provide a message editor for the resulting commit.
- Git worktrees are not supported yet.
- Bare repositories are not supported.
- Submodules are ignored in commit diffs.
- Merge commit file lists and file diffs are compared to the first parent only.
- The graph renderer models ordinary two-parent merges; octopus merges are not a first-class display target.
- Tags are lightweight only. There is no annotated-tag message flow and no remote tag deletion flow.
- Branch rename, remote management, and branch upstream editing are not implemented.
- Search only matches loaded commit SHA prefixes. It does not search commit messages, authors, branches, tags, filenames, or unloaded history.
- The keymap is customizable by editing `keymap.json`, but there is no in-app keymap editor.
- The recent repository list is append-only from inside the app; there is no in-app remove/reorder UI.
- Theme selection is available in settings, but theme choice is not currently persisted between launches.

### Roadmap

Planned or desired features include jujutsu integration, Git worktrees, conflict mode, merging, rebasing, and more.

Follow the project board for current work:

[https://github.com/users/asinglebit/projects/1/views/1](https://github.com/users/asinglebit/projects/1/views/1)

### Default Keyboard Mappings

You will probably want to tune the keymap for your terminal and OS, especially on macOS where Option/Command behavior varies by terminal. Defaults are written to `keymap.json` on first run.

Dangerous actions live behind action mode. By default, press `Ctrl+a`, then press the action key. Action mode is single-shot, so each dangerous command needs a fresh prefix.

#### Normal Mode

| Command | Default keys |
| --- | --- |
| Widen Scope | `h`, `Left` |
| Narrow Scope | `l`, `Right` |
| Select | `Enter` |
| Back | `Esc` |
| Focus Previous Pane | `Ctrl+p`, `Shift+BackTab` |
| Focus Next Pane | `Ctrl+n`, `Tab` |
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
| Scroll Up Branch | `{` |
| Scroll Down Branch | `}` |
| Scroll Up Commit | `[` |
| Scroll Down Commit | `]` |
| Toggle Hunk Mode | `m` |
| Toggle Split Diff Mode | `v` |
| Toggle Zen Mode | `z` |
| Toggle Branches | `1` |
| Toggle Tags | `2` |
| Toggle Stashes | `3` |
| Toggle Status | `4` |
| Toggle Inspector | `5` |
| Toggle SHAs | `6` |
| Toggle Help / Settings | `?` |
| Action Mode | `Ctrl+a` |
| Minimize | `.` |
| Reload | `r` |
| Exit | `q` |
| Stage | `s` |
| Unstage | `u` |
| Commit | `c` |
| Fetch All | `f` |
| Create Branch | `b` |
| Tag | `t` |
| Toggle Branch | `Shift+T` |
| Solo Branch | `Space` |

#### Action Mode

| Command | Default key after `Ctrl+a` |
| --- | --- |
| Drop | `x` |
| Pop | `p` |
| Stash | `Shift+S` |
| Checkout | `o` |
| Hard Reset | `Shift+H` |
| Mixed Reset | `Shift+M` |
| Force Push | `Shift+P` |
| Push Tags | `Shift+V` |
| Delete Branch | `Shift+D` |
| Untag | `Shift+U` |
| Cherrypick | `y` |

### Screenshots

![1](https://github.com/user-attachments/assets/37df457b-bbf4-4d51-a965-c300b426cb62)
![1](https://github.com/user-attachments/assets/b09f73e7-5bda-4ecd-ba7c-3bba85395e37)
![6](https://github.com/user-attachments/assets/2ed8f14e-193b-4815-b37e-283bd129787f)
![5](https://github.com/user-attachments/assets/15e4630f-a141-4724-9d35-1b8601006598)
![4](https://github.com/user-attachments/assets/10389ec5-6780-4bcb-85dc-67f9e012ed63)
![3](https://github.com/user-attachments/assets/a408af0c-ef85-4692-914b-81562d3873e4)
![untitled](https://github.com/user-attachments/assets/72d09d11-86cb-4356-a3dd-93684abc9b19)
