# Graph Internals

This document explains how `guitar` turns Git history into the graph column shown in
the terminal UI. It is written for future maintainers who need to debug lane
placement, merge rendering, window alignment, or odd glyphs.

The short version:

```text
Git refs / reflog roots
        |
        v
Batcher revwalk pages
        |
        v
Walker: assign aliases, read parents, update lanes
        |
        v
Buffer: store lane snapshots as deltas
        |
        v
GraphService: project rows + visible history window
        |
        v
render_graph_projection: turn snapshots into layered symbols
        |
        v
App::draw_graph: align cached projections into the visible table
```

The graph is topology-first. The renderer does not ask Git where to draw a
line. Instead, the walker records enough lane state for each commit row, and
the renderer derives visible pipes, merge curves, branch curves, and commit
markers from the current and previous lane snapshots.

## Vocabulary

Most diagrams in this file use portable ASCII:

```text
o   normal commit
*   merge commit
@   uncommitted pseudo-row
|   continuing lane
-   horizontal merge/branch connector
/   conceptual edge moving left
\   conceptual edge moving right
.   empty lane
```

The real UI uses Unicode glyphs from `src/helpers/symbols.rs`:

| Concept | Constant | Glyph |
| --- | --- | --- |
| normal commit | `graph::COMMIT` | `○` |
| commit with branch label | `graph::COMMIT_BRANCH` | `●` |
| stash commit | `graph::COMMIT_STASH` | `◎` |
| merge commit | `graph::MERGE` | `•` |
| uncommitted row | `graph::UNCOMMITTED` | `◌` |
| vertical lane | `graph::VERTICAL` | `│` |
| vertical lane to HEAD from uncommitted row | `graph::VERTICAL_DOTTED` | `┊` |
| horizontal connector | `graph::HORIZONTAL` | `─` |
| merge from left | `graph::MERGE_LEFT_FROM` | `┤` |
| merge from right | `graph::MERGE_RIGHT_FROM` | `╭` |
| branch up | `graph::BRANCH_UP` | `╯` |
| branch down | `graph::BRANCH_DOWN` | `╮` |

## Source Files

The graph pipeline is split across a few focused modules:

| File | Role |
| --- | --- |
| `src/core/batcher.rs` | Owns the libgit2 revwalk and returns pages of OIDs. |
| `src/core/oids.rs` | Maps Git OIDs to compact `u32` aliases and stores rendered order. |
| `src/core/chunk.rs` | Defines the lane entry stored in the graph buffer. |
| `src/core/buffer.rs` | Maintains current lanes and records deltas/checkpoints. |
| `src/core/walker.rs` | Reads commits, parents, refs, stashes, reflogs, and updates the buffer. |
| `src/core/graph_service.rs` | Runs the walker in a worker thread and projects visible windows. |
| `src/core/renderers.rs` | Renders graph, SHA, date, committer, and message projections. |
| `src/core/layers.rs` | Composites commit, merge, and pipe layers into terminal spans. |
| `src/app/draw/graph.rs` | Requests/caches graph windows and draws the final table. |

## High-Level Pipeline

### 1. Roots become a revwalk

`Batcher::new` builds a libgit2 revwalk from:

- visible local branch tips,
- visible remote branch tips,
- optional HEAD reflog roots.

Hidden branch names are a deny-list. Hidden refs are not pushed into the
revwalk, but they can still appear in side panes and metadata.

```text
refs/heads/main        M
refs/heads/feature     F
refs/remotes/origin/x  X
HEAD reflog root       R   (only when graph reflogs are enabled)

          visible roots
               |
               v
        libgit2 revwalk
   Sort::TOPOLOGICAL | Sort::TIME
```

The worker repeatedly calls `Batcher::next(amount)`. Each page becomes a
`sorted_batch` of aliases in `Walker::walk`.

### 2. OIDs become aliases

The UI does not pass full SHA-1/SHA-256 strings through every graph data
structure. `Oids` assigns compact aliases:

```text
Oid("abcd...") -> alias 0
Oid("1234...") -> alias 1
Oid("beef...") -> alias 2
```

`NONE` is `u32::MAX`. It is used as:

- the synthetic uncommitted row alias,
- "no parent",
- dummy lane alias.

`Oids::sorted_aliases` starts as:

```text
[NONE]
```

That first row is the uncommitted-work pseudo-row above HEAD. Real commits are
appended as the walker sees them.

### 3. Commits become chunks

For each commit alias in the current revwalk page, `Walker` reads at most two
parents:

```text
Chunk::commit(alias, parent_a, parent_b)
```

Only two parents are modeled. For octopus merges, later parents are ignored by
the current graph renderer.

Stashes are special: stash commits are real Git commits with extra parents for
index/worktree state, but the graph intentionally keeps only the first parent
so the stash reads as a side snapshot near its base commit:

```text
normal commit:  Chunk { alias: C, parent_a: P1, parent_b: P2 }
stash commit:   Chunk { alias: S, parent_a: base, parent_b: NONE }
root commit:    Chunk { alias: R, parent_a: NONE, parent_b: NONE }
```

### 4. Chunks update lanes

`Buffer` owns the active lane list:

```text
Buffer.curr = [
  lane 0: Chunk { alias: 10, parent_a: 8, parent_b: NONE, marker: Commit },
  lane 1: Chunk { alias: 11, parent_a: 7, parent_b: NONE, marker: Commit },
]
```

Each lane says:

- which commit currently occupies the lane (`alias`),
- which parent alias the lane wants to meet next (`parent_a`),
- optionally a second parent for one merge edge (`parent_b`),
- whether the lane is a commit, dummy, or uncommitted marker.

The buffer prefers to replace a lane whose `parent_a` equals the incoming
commit alias. If none exists, the incoming commit starts a new rightmost lane.

```text
Before update with alias B:

lane:      0
curr:     A -> B

After update with B:

lane:      0
curr:     B -> C
```

If no lane points at the incoming alias:

```text
Before update with alias X:

lane:      0
curr:     A -> B

After update with X:

lane:      0       1
curr:     A -> B   X -> Y
```

### 5. Buffer snapshots become graph windows

The full graph can be huge, so `Buffer` does not store every snapshot as a
full copy. It stores deltas plus occasional checkpoints:

```text
DeltaOp::Insert  { index, item }
DeltaOp::Remove  { index }
DeltaOp::Replace { index, new }
```

`Buffer::window(start, end)` reconstructs only the snapshots needed for a
visible request:

```text
nearest checkpoint before start
        |
        v
replay deltas from checkpoint+1 to end
        |
        v
GraphHistory = Vector<Vector<Chunk>>
```

`GraphService` asks for:

```text
buffer.window(start, end + 1)
```

That extra requested snapshot gives the renderer enough surrounding history to
align rows and compare current/previous lane states.

### 6. Rows and history are sent together

`GraphService` sends a `GraphWindow` event:

```text
GraphWindow {
  start,
  end,
  total,
  head_alias,
  rows: Vec<GraphRow>,
  history: GraphHistory,
}
```

`GraphRow` contains display metadata, not topology:

```text
GraphRow {
  index,
  alias,
  oid,
  summary,
  committer_date,
  committer_name,
  has_any_branch,
  branches,
  tags,
  is_stash,
  stash_lane,
  worktrees,
  reflog,
}
```

Topology lives in `history`. Labels and row text live in `rows`.

### 7. Renderer turns snapshots into symbols

`render_graph_projection` receives:

- `rows`,
- `history`,
- `head_alias`,
- cached `start`/`end`,
- whether the uncommitted row is stable enough to render.

For each row, it computes a history index:

```rust
let delta = (history.len() + global_idx).saturating_sub(end);
let prev = if delta == 0 { None } else { history.get(delta - 1) };
let last = history.get(delta);
```

Do not assume `history[0]` is always `rows[0]`. The window can be reconstructed
from an earlier checkpoint, and the renderer uses `global_idx` and cached
`end` to align row indices to snapshots.

### 8. App draws cached projections

`App::draw_graph`:

1. computes the visible graph range,
2. requests a preloaded window one screen before and after,
3. renders all projections from the cached window,
4. aligns those projections back into the visible range,
5. builds the final `ratatui::Table`.

```text
visible rows:     [20 ........ 39]
requested rows:   [ 0 ........ 59]  (one screen before/after)
cached rows:      [ 0 ........ 59]
drawn rows:       [20 ........ 39]
```

The graph can draw while history is still loading. The uncommitted row is held
back until `graph_window_has_stable_visible_page` says the cached rows and
history fully cover the visible page.

## Memory Model

### Chunk

`Chunk` is the unit of lane topology:

```text
Chunk
+----------+------------------------------------------+
| alias    | the commit occupying this lane           |
| parent_a | the main parent this lane should meet    |
| parent_b | optional second parent for merge drawing |
| marker   | Uncommitted | Commit | Dummy             |
+----------+------------------------------------------+
```

Examples:

```text
linear commit:
  Chunk { alias: A, parent_a: B, parent_b: NONE, marker: Commit }

merge commit:
  Chunk { alias: M, parent_a: L, parent_b: R, marker: Commit }

root commit:
  Chunk { alias: R, parent_a: NONE, parent_b: NONE, marker: Commit }

dummy lane:
  Chunk { alias: NONE, parent_a: NONE, parent_b: NONE, marker: Dummy }

uncommitted row:
  Chunk { alias: NONE, parent_a: HEAD, parent_b: NONE, marker: Uncommitted }
```

### Buffer.curr

`Buffer.curr` is the active lane state after the latest update:

```text
lane index:    0                  1                  2
Buffer.curr:  A -> B             X -> Y             M -> P,Q
```

A lane can be active even when its commit is not the row being rendered. In that
case the renderer usually draws a vertical pipe:

```text
row alias A:

lane 0: row commit A      -> draw commit marker
lane 1: active X -> Y     -> draw "|"
lane 2: active M -> P,Q   -> draw "|", unless merge rendering overlays it
```

### Deltas and checkpoints

Each `Buffer::update` starts with `backup()`. That moves the previous delta into
`deltas`, then the current update records new operations.

```text
update(A):
  backup previous delta
  mutate curr
  record delta ops for A

update(B):
  backup A's delta
  mutate curr
  record delta ops for B
```

This makes row snapshots reproducible without storing every full `curr`:

```text
checkpoint[0] = []

delta[1] = Insert lane 0: uncommitted
delta[2] = Replace lane 0: A -> B
delta[3] = Replace lane 0: B -> C

window(0, 4) replays:
  [] + delta[1] => snapshot row 0
     + delta[2] => snapshot row 1
     + delta[3] => snapshot row 2
```

Checkpoints are stored every 100 deltas so reconstructing a far-down window
does not require replaying the whole graph.

### Lane index vs visual columns

Lane index is mostly a topology/color concept. Rendering happens by pushing
symbols into layers. Each lane usually emits two graph tokens:

```text
lane index:       0        1        2
token pair:     [0][1]   [2][3]   [4][5]
ASCII idea:      o        |        |
```

The `lane_idx` passed into `layers.commit`, `layers.merge`, and `layers.pipe`
selects the lane color. The visual column position is produced by call order.
That is why renderer code must emit empty placeholders for lanes that should
occupy space but not show a glyph.

### Layers

The renderer builds three layers:

```text
Commits  -> commit dots, branch dots, stash dots
Merges   -> horizontal merge/branch connectors
Pipes    -> vertical pipes and branch-up curves
```

`LayersContext::bake` composites them in this priority order:

```text
Commits wins over Merges wins over Pipes
```

If a merge connector crosses under a commit marker, the commit marker remains
visible.

```text
commit layer:    .  o  .
merge layer:     -----
pipe layer:      |  |  |
baked result:    -  o  -
```

After baking, `remove_empty_columns` prunes lane pairs that are visually empty
across the rendered window. Empty or horizontal-only pairs do not keep a column
alive; commits, verticals, and curve glyphs do.

## Walker Passes

`Walker::walk` processes one revwalk page.

### Initialization pass

`Walker::new` seeds metadata that should be stable before walking:

```text
open Repository
create Buffer
create Oids
collect branch tip aliases
collect tag aliases
collect stash aliases
collect HEAD reflog entries
optionally add reflog roots to Batcher
create Batcher
```

Branch and tag tips are registered early so aliases stay consistent between
refs and commits.

### Page pass

For each `walk()` call:

```text
read HEAD
fetch sorted_batch from Batcher
insert uncommitted row once, before first real commit
insert stashes near their base parent in sorted_batch
for each alias in sorted_batch:
  read commit
  read first two parents
  simplify stash parents
  buffer.update(chunk)
  record ref/tag/stash/reflog lane metadata
  detect merge caveats
  append alias to sorted_aliases
backup once more when the revwalk is empty
```

### Ref lane metadata pass

After `buffer.update(chunk)`, the walker scans `buffer.curr` to find the lane
where the alias landed. It records lane numbers for:

- branch labels,
- tag labels,
- stash labels,
- reflog labels.

Those lane numbers are used for label colors in message/pane rendering. They
do not drive topology.

## Buffer Update Rules

### Rule: expire transient lanes first

At the start of `Buffer::update`, after `backup()`, any lane scheduled by
`expire_lane_after_snapshot` becomes a dummy lane. Then trailing dummies are
removed.

This is important for merge commits that should exist for exactly one row.

```text
row M snapshot:

lane:       0       1       2
curr:      A->P    B->Q    M->P,Q
render:    |       |       *

next update starts:

lane 2 is transient -> dummy -> trailing dummy removed

lane:       0       1
curr:      A->P    B->Q
render:    |       |
```

### Rule: remove trailing dummy lanes

Trailing dummy lanes carry no future topology and can collapse immediately:

```text
before:
lane:      0       1       2
curr:     A->B    X->Y    dummy

after:
lane:      0       1
curr:     A->B    X->Y
```

Interior dummy lanes can remain temporarily because they preserve spacing
between active lanes.

### Rule: split planned mergers

If the walker sees a two-parent merge whose second parent is not already
visible as a lane, it calls `buffer.merger(alias)`.

On the next update, `Buffer` finds that merge alias and splits it:

```text
After row M:

lane:      0
curr:     M -> A,B
mergers:  [M]

Before next row is applied:

lane:      0          1
curr:     M -> A      M -> B
```

The original lane keeps `parent_a`; the cloned lane carries `parent_b`. This
creates a lane for the second parent so later rows can draw the merge edge.

### Rule: replace matching parent lane

If an incoming commit alias matches an existing lane's `parent_a`, that lane is
replaced by the incoming commit:

```text
incoming: B -> C

before:
lane 0: A -> B

after:
lane 0: B -> C
```

Then all other lanes that pointed at the consumed alias clear that parent. If a
lane loses all parents, it becomes dummy:

```text
before:
lane 0: B -> C
lane 1: X -> B

after B is consumed:
lane 0: B -> C
lane 1: dummy
```

### Rule: append otherwise

If no lane is waiting for the incoming alias, the commit starts a new rightmost
lane:

```text
incoming: X -> Y

before:
lane 0: A -> B

after:
lane 0: A -> B
lane 1: X -> Y
```

## Renderer Passes

`render_graph_projection` is the main graph-symbol renderer. It renders only
the graph column. SHA/date/committer/message projections are rendered by
separate functions and assembled later by `App::draw_graph`.

For each `GraphRow`, the renderer does these passes.

### Pass 1: find current and previous snapshots

```text
global_idx = row.index
delta = (history.len() + global_idx).saturating_sub(end)
prev = history[delta - 1], if any
last = history[delta]
```

`last` is the lane state for the row. `prev` is used to decide whether a dummy
lane should draw a branch-up curve.

### Pass 2: handle uncommitted row

If the row alias is `NONE` and the page is stable, render only the uncommitted
marker:

```text
@  working tree changes
```

The actual glyph is `◌`. The row is skipped while the cached graph window is
not stable enough to show it.

### Pass 3: pre-scan branching lanes

Before drawing lanes, the renderer collects `branching_lanes`. These are dummy
lanes in the current snapshot that need a branch-up curve because the previous
snapshot showed an active single-parent lane or because the lane newly
appeared.

```text
prev:
lane 0: A -> B
lane 1: X -> Y

last:
lane 0: A -> B
lane 1: dummy

renderer:
lane 1 gets branch-up curve
```

ASCII idea:

```text
o  |
|  o
| /
o
```

### Pass 4: walk each lane in the current snapshot

For each `chunk` in `last`:

```text
if chunk is dummy:
  maybe draw branch-up
else if chunk.alias == row.alias:
  draw row commit marker
  maybe draw merge connector
else:
  draw vertical pipe if the lane still has parents
```

### Pass 5: fallback commit marker

If the row alias was not found in the snapshot, the renderer still draws a
commit marker at the next lane. This keeps rows from going visually blank when
the projected row list is ahead of the available lane history.

### Pass 6: bake and prune

The layer context bakes commit/merge/pipe layers into spans, then
`remove_empty_columns` removes graph lane pairs that are empty across every
rendered row.

## Commit Marker Precedence

When the row alias is found, marker selection is:

```text
if two parents and no branch label:
  graph::MERGE
else if any branch label:
  graph::COMMIT_BRANCH
else if any matching worktree label:
  worktree::CURRENT
else if stash:
  graph::COMMIT_STASH
else:
  graph::COMMIT
```

Examples:

```text
two-parent merge without branch:   *
branch tip, even if merge:         o  (actual glyph is filled branch dot)
worktree commit without branch:    w  (actual glyph is worktree icon)
stash commit:                      s  (actual glyph is ◎)
ordinary commit:                   o
```

Branch labels have higher precedence than the merge marker. This is why a
merge commit at a branch tip can show the branch dot instead of the merge dot.

## Caveats And Examples

### Linear history

Git topology:

```text
A
|
B
|
C
```

Aliases:

```text
A = 0
B = 1
C = 2
```

Lane snapshots:

```text
row  lane 0
@    NONE -> A       uncommitted pseudo-row
A    A -> B
B    B -> C
C    C -> NONE
```

ASCII render:

```text
@  working tree
o  A
|
o  B
|
o  C
```

Actual graph column uses `◌`, `○`, and `│`.

### Branch continuation

Topology:

```text
A
|
B     X
|     |
C     Y
 \   /
   R
```

During rows for `A`, `B`, and `C`, the side branch lane remains active:

```text
row B snapshot:

lane 0: B -> C
lane 1: X -> Y
```

Rendering row `B`:

```text
o  |   B row marker on lane 0, side lane continues
```

The side lane draws a vertical pipe because its chunk is not the current row
and still has a parent.

### Dummy lane and branch-up

A dummy lane means topology in that lane has just ended or collapsed. The
renderer checks the previous snapshot to decide if it should show a curve.

```text
prev:
lane 0: A -> B
lane 1: X -> Y

last:
lane 0: A -> B
lane 1: dummy
```

ASCII idea:

```text
o  |
|  o
| /
o
```

The curve is rendered with `graph::BRANCH_UP` (`╯`) in the pipe layer.

### Visible-parent merge

The simplest merge row is when the second parent is already visible as a
single-parent lane.

Topology:

```text
M
|\
L R
```

Snapshot at row `M`:

```text
lane 0: R -> P
lane 1: M -> L,R
```

Renderer sees:

```text
M.parent_b == R
lane 0 parent_a == R
```

ASCII render:

```text
o---*  M
|   |
o   o  L/R later
```

Actual connector pieces are layered with `MERGE_RIGHT_FROM`, `HORIZONTAL`, and `MERGE_LEFT_FROM` depending on
direction and position.

When a merge connector starts from a lane that was already carrying the same
parent on the previous scanline, the renderer uses `MERGE_RIGHT_FROM`
instead of a plain horizontal. ASCII idea:

```text
previous scanline:   |  lane carries parent P upward for another commit
current scanline:    +--*  merge also starts from parent P
```

The actual `+` shape is `├`. This keeps the connector from visually erasing the
upward continuation of that parent lane.

### Deferred merge lane split

If the second parent is not already visible, the walker schedules a deferred
merge:

```text
row M:
lane 0: M -> L,R

R is not visible as any lane parent.
Walker calls buffer.merger(M).
```

Before the next update, the buffer splits the lane:

```text
lane 0: M -> L
lane 1: M -> R
```

ASCII idea:

```text
*  M
|\
| \
o  o  L/R appear later
```

Without this split, the renderer would not have a lane to carry the second
parent downward.

### New rightmost transient merge lane

This is the subtle case fixed by the transient-lane behavior.

Topology:

```text
T1     T2
|      |
P1     P2
 \    /
   M
```

Revwalk order can make `M` appear on a new rightmost lane after both parent
lanes are already visible:

```text
before M:

lane 0: T1 -> P1
lane 1: T2 -> P2

after M is appended:

lane 0: T1 -> P1
lane 1: T2 -> P2
lane 2: M  -> P1,P2
```

The merge row needs lane 2 so the merge dot can be drawn:

```text
|  |  *  M
```

But lane 2 must not continue on the next scanline, because neither parent is
on lane 2. Both parents are already represented by previous lanes.

Wrong next row:

```text
|  |  |    <- false pipe under M
```

Correct next row:

```text
|  |
```

The walker detects the case when:

- the commit has two parents,
- the update started a new lane,
- that lane is rightmost,
- both parents are already present on prior lanes,
- no deferred merge split is needed.

It then calls `buffer.expire_lane_after_snapshot(lane_idx)`. The buffer keeps
the lane for the merge row snapshot and removes it at the start of the next
update.

### Uncommitted pseudo-row

The first rendered row is not a Git commit:

```text
alias: NONE
chunk: Chunk::uncommitted(head_alias, NONE)
```

ASCII:

```text
@  staged/unstaged/conflict summary
|
o  HEAD
```

When an active lane points at `HEAD` from lane 0, the renderer can draw a
dotted vertical pipe (`┊`) to visually distinguish the working-tree row from
normal commit history.

### Stash parent simplification

Git stash commits usually have multiple parents:

```text
stash commit
|\
| \ index/worktree state
base
```

The graph intentionally renders only the base parent:

```text
S -> base
```

Stashes are inserted near their first parent in the sorted batch:

```text
... parent ...
    stash
... next ...
```

This avoids drawing index/worktree internals as normal history.

### Optional HEAD reflog roots

HEAD reflog entries are always collected for labels. When graph reflogs are
enabled, their new OIDs are also pushed as extra revwalk roots.

Without reflog roots:

```text
main -> A -> B

lost commit L appears only in HEAD reflog
L is not walked
```

With reflog roots:

```text
roots: main, L

main -> A -> B
L    -> ...
```

`GraphRow.reflog` stores the latest reflog label for an alias. Reflog lanes are
recorded after `Buffer` decides where the alias landed.

### Hidden branches and labels

Hidden branch names affect roots:

```text
visible branch -> pushed into revwalk
hidden branch  -> skipped by Batcher roots
```

Labels are filtered separately when `graph_rows` builds `GraphRow.branches`.
A hidden branch can still appear in the branch pane, but it does not contribute
to graph roots and is not rendered as a visible graph label.

### First-two-parents limitation

The walker currently reads:

```text
parent_a = first parent
parent_b = second parent
ignore parent_c, parent_d, ...
```

An octopus merge:

```text
M
|\ \ \
A B C D
```

is modeled as:

```text
M
|\
A B
```

This is a deliberate renderer limitation, not a Git limitation.

### Commit not found in snapshot

The renderer has a fallback when `GraphRow.alias` is not found in `last`.
It draws a commit marker at the next lane. This can happen while the UI has a
row projection but not enough lane history yet.

```text
row exists
history missing alias
        |
        v
draw marker instead of blank row
```

This is a graceful loading behavior.

## Window Alignment

`GraphService` and `App::draw_graph` use windows at different layers.

### Worker window

The worker clamps requests to known commit count:

```text
start = min(requested_start, total)
end   = min(requested_end, total)
```

It sends:

```text
rows    = graph_rows(start, end)
history = buffer.window(start, end + 1)
```

### UI preload window

The UI requests more than it currently shows:

```text
visible_height = 20
visible        = [100..120)
preload        = [80..140)
```

This keeps scrolling smooth because nearby rows are already cached.

### Projection alignment

Projection functions render the cached window. `align_projection` then maps the
cached lines into the current visible range:

```text
cached_start = 80
target_start = 100
target_end   = 120

target row 100 uses cached line 20
target row 101 uses cached line 21
...
```

If the requested visible page is not fully covered yet, blank lines are used
until the worker sends a better window.

## Rendering Examples

### Linear row rendering

Snapshot:

```text
row B

lane 0: B -> C
lane 1: X -> Y
```

Renderer output:

```text
o  |   B
```

Layer calls, conceptually:

```text
lane 0 current row:
  commit layer: o .
  pipe layer:   . .

lane 1 other active chunk:
  commit layer: . .
  pipe layer:   | .
```

### Merge connector rendering

Snapshot:

```text
row M

lane 0: R -> P
lane 1: L -> P
lane 2: M -> L,R
```

ASCII:

```text
|  |  *   M
|--|--/
o  o      L/R later
```

The real renderer uses the merge layer to draw connector glyphs and the commit
layer to keep the merge dot visible.

If lane 0 was already carrying `R` on the previous scanline, the first
connector piece can be `MERGE_RIGHT_FROM` instead of a plain
horizontal. That says "start the merge to the right, while this same parent
also continues upward from the prior row."

### Empty-column pruning

Before pruning:

```text
col pair:  0   1   2
row A:     o   -   -
row B:     |   -   -
row C:     o   -   -
```

If pairs 1 and 2 contain only spaces or horizontals across the whole rendered
window, `remove_empty_columns` removes them:

```text
row A:     o
row B:     |
row C:     o
```

This keeps the graph column compact.

## Debugging Checklist

When a graph bug appears, inspect the pipeline in this order:

1. `Oids::sorted_aliases`
   - Does the row exist?
   - Is `NONE` still row 0?
   - Did the revwalk include or exclude the expected hidden/reflog roots?

2. Parent aliases
   - Does the commit have the expected `parent_a` and `parent_b`?
   - Is it a stash, and therefore intentionally simplified to one parent?
   - Is it an octopus merge, and therefore truncated to two parents?

3. `Buffer.curr` after `buffer.update`
   - Did the commit replace an existing parent lane?
   - Did it start a new lane?
   - Did another lane clear a consumed parent and become dummy?

4. Merge scheduling
   - If `parent_b` was not visible, was `buffer.merger(alias)` scheduled?
   - If both parents were visible on previous lanes, was the new rightmost lane
     marked transient instead?

5. Deltas and windows
   - Does `Buffer::window(start, end + 1)` contain the row snapshot?
   - Is the renderer's `delta` mapping pointing at the expected snapshot?
   - Does `prev` contain the lane state needed for branch-up rendering?

6. Renderer row pass
   - Is the current row alias found in `last`?
   - Is an unrelated active lane drawing a vertical pipe because it still has a parent?
   - Is a dummy lane drawing `BRANCH_UP` because `prev` had a single-parent lane?

7. Layer compositing
   - Is a commit marker hiding a merge/pipe glyph as intended?
   - Is the correct lane color being selected?
   - Are enough empty placeholders emitted to preserve visual spacing?

8. Empty-column pruning
   - Did a column disappear because it contained only spaces and horizontals?
   - Is the bug only visible for a narrow cached window?

9. App cache alignment
   - Is the cached window overlapping the visible range?
   - Is `align_projection` shifting rows because the user scrolled before the
     worker returned a new window?
   - Is the uncommitted row hidden because the cached page is not stable yet?

## Quick Mental Models

### A lane is a promise

```text
lane 0: A -> B
```

means:

```text
"Draw A here now. Keep this lane alive until B appears."
```

### A dummy is a recently closed promise

```text
lane 1: dummy
prev lane 1: X -> Y
```

means:

```text
"This lane just closed. Draw a curve if needed, then let it collapse."
```

### A merge is two promises in one chunk

```text
M -> A,B
```

means:

```text
"M's first-parent lane goes to A, and one merge edge also needs to reach B."
```

If `B` is not already represented by a lane, the buffer can split `M` into two
lane entries on the next update.

### A transient lane is a marker-only promise

```text
M -> A,B
```

on a new rightmost lane can mean:

```text
"Draw M here, but do not continue this lane. A and B already have lanes."
```

That is what `expire_lane_after_snapshot` records.
