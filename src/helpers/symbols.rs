pub mod branch {
    pub const LOCAL_VISIBLE: &str = "●";
    pub const LOCAL_HIDDEN: &str = "○";
    pub const REMOTE_VISIBLE: &str = "◆";
    pub const REMOTE_HIDDEN: &str = "◇";
}

pub mod border {
    pub const HORIZONTAL: &str = "─";
    pub const VERTICAL: &str = "│";
    pub const T_RIGHT: &str = "┤";
    pub const TOP_T: &str = "┬";
    pub const BOTTOM_T: &str = "┴";
    pub const ROUNDED_TOP_RIGHT: &str = "╮";
    pub const ROUNDED_BOTTOM_RIGHT: &str = "╯";
    pub const ROUNDED_TOP_LEFT: &str = "╭";
}

pub mod entity {
    pub const FOLDER: &str = "";
    pub const TAG: &str = "\u{F04F9}";
    pub const REFLOG: &str = "\u{F0E2}";
}

pub mod empty_state {
    pub const MARK: &str = "⊘";
}

pub mod form {
    pub const CHECKBOX_OFF: &str = "🞎";
    pub const CHECKBOX_ON: &str = "🞕";
    pub const RADIO_OFF: &str = "🞅";
    pub const RADIO_ON: &str = "🞊";
}

pub mod graph {
    pub const COMMIT_BRANCH: &str = super::branch::LOCAL_VISIBLE;
    pub const COMMIT: &str = super::branch::LOCAL_HIDDEN;
    pub const COMMIT_STASH: &str = "◎";
    pub const EMPTY: &str = " ";
    pub const HORIZONTAL: &str = super::border::HORIZONTAL;
    pub const VERTICAL: &str = super::border::VERTICAL;
    pub const VERTICAL_DOTTED: &str = "┊";
    pub const MERGE_LEFT_FROM: &str = super::border::T_RIGHT;
    pub const MERGE_RIGHT_FROM: &str = super::border::ROUNDED_TOP_LEFT;
    pub const BRANCH_UP: &str = super::border::ROUNDED_BOTTOM_RIGHT;
    pub const BRANCH_DOWN: &str = super::border::ROUNDED_TOP_RIGHT;
    pub const MERGE: &str = "•";
    pub const UNCOMMITTED: &str = "◌";
}

pub mod modal {
    pub const SELECTED: &str = ">";
    pub const UNSELECTED: &str = " ";
    pub const MASK: &str = "*";
}

pub mod scrollbar {
    pub const BEGIN: &str = super::border::ROUNDED_TOP_RIGHT;
    pub const END: &str = super::border::ROUNDED_BOTTOM_RIGHT;
    pub const TRACK: &str = super::border::VERTICAL;
    pub const THUMB: &str = "▌";
    pub const INACTIVE_THUMB: &str = super::border::VERTICAL;
}

pub mod settings {
    pub const COMPACT_TAB: &str = "•";
}

pub mod splash {
    pub const LOGO_WORD_PREFIX: &str = "  guita";
    pub const LOGO_COMPACT: &str = "guita╭";
    pub const LOGO_NARROW: [&str; 9] = [
        "                    :#   :#                 ",
        "                         L#                 ",
        "  .##5#^.  .#   .#  :C  #C6#   #?##:        ",
        "  #B   #G  C#   #B  #7   B?        G#       ",
        "  #4   B5  B5   B5  B5   B5    1B5B#G  .a###",
        "  #b   5?  ?B   B5  B5   B5   ##   ##  B?   ",
        "  .#B~6G!  .#6#~G.  #5   ~##  .##Y~#.  !#   ",
        "      .##                              !B   ",
        "     ~G#                               ~?   ",
    ];
    pub const LOGO_WIDE: [&str; 11] = [
        "                                 :GG~        .?Y.                                ",
        "       ....        ..      ..   .....      . ^BG: ..       .....                 ",
        "    .7555YY7JP^   ~PJ     ~PJ  ?YY5PP~    7YY5BGYYYYJ.   J555YY557.              ",
        "   .5B?.  :JBB~   !#5     !#5  ...PB~     ...^BG:....    ~:.   .7#5           :^^",
        "   7#5     .GB~   !B5     !B5     PB~        :BG.        .~7??J?JBG:      .~JPPPY",
        "   ?#Y      PB~   !B5     !B5     PB~        :BG.       7GP7~^^^!BG:     ~5GY!:. ",
        "   ^GB~    7BB~   ^BG.   .YB5     5#7        :BB:       P#!     JBG:    ^GG7     ",
        "    ^5G5JJYJPB~    JBP???YYB5     ^5GYJJ?.    7GPJ???.  ~PGJ77?5J5B!    JG5      ",
        "      .^~^..GB:     :~!!~. ^^       :~~~~      .^~~~~    .^!!!~. .^:    JG5      ",
        "    .?!^^^!5G7                                                          YB5      ",
        "    .!?JJJ?!:                                                           75?      ",
    ];
    pub const SELECTED_LEFT: &str = "⏵ ";
    pub const SELECTED_RIGHT: &str = " ⏴";
    pub const LOGO_CORNER: &str = super::border::ROUNDED_TOP_LEFT;
}

pub mod status {
    pub const ADDED: &str = "+";
    pub const ADDED_SPACED: &str = "+ ";
    pub const CONFLICT: &str = "!";
    pub const CONFLICT_SPACED: &str = "! ";
    pub const DELETED: &str = "-";
    pub const DELETED_SPACED: &str = "- ";
    pub const MODIFIED: &str = "~";
    pub const MODIFIED_SPACED: &str = "~ ";
    pub const RENAMED: &str = ">";
    pub const RENAMED_ARROW_SPACED: &str = "→ ";
    pub const OTHER: &str = "*";
    pub const OTHER_SPACED: &str = "  ";
}

pub mod submodule {
    pub const DEFAULT: &str = "\u{F03D6}";
    pub const EMPTY: &str = "\u{F19D9}";
    pub const DIRTY: &str = "\u{F19D8}";
    pub const UNINITIALIZED: &str = "\u{f487}";
    pub const STACK_SEPARATOR: &str = " › ";
}

pub mod worktree {
    pub const CURRENT: &str = "\u{F0770}";
    pub const OTHER: &str = "\u{F0256}";
    pub const DIRTY: &str = "\u{F0DCE}";
    pub const LOCKED: &str = "\u{F1AA8}";
    pub const INVALID: &str = "\u{F19F9}";
    pub const EMPTY: &str = "\u{F179E}";
}

pub const WEEKDAY_LABELS: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];

pub const SYM_FOLDER: &str = entity::FOLDER;
pub const SYM_COMMIT_BRANCH: &str = graph::COMMIT_BRANCH;
pub const SYM_COMMIT: &str = graph::COMMIT;
pub const SYM_COMMIT_STASH: &str = graph::COMMIT_STASH;
pub const SYM_REFLOG: &str = entity::REFLOG;
pub const SYM_WORKTREE: &str = worktree::CURRENT;
pub const SYM_WORKTREE_OTHER: &str = worktree::OTHER;
pub const SYM_WORKTREE_DIRTY: &str = worktree::DIRTY;
pub const SYM_WORKTREE_LOCKED: &str = worktree::LOCKED;
pub const SYM_WORKTREE_INVALID: &str = worktree::INVALID;
pub const SYM_WORKTREE_EMPTY: &str = worktree::EMPTY;
pub const SYM_SUBMODULE: &str = submodule::DEFAULT;
pub const SYM_SUBMODULE_EMPTY: &str = submodule::EMPTY;
pub const SYM_SUBMODULE_DIRTY: &str = submodule::DIRTY;
pub const SYM_SUBMODULE_UNINITIALIZED: &str = submodule::UNINITIALIZED;
pub const SYM_VERTICAL: &str = graph::VERTICAL;
pub const SYM_VERTICAL_DOTTED: &str = graph::VERTICAL_DOTTED;
pub const SYM_HORIZONTAL: &str = graph::HORIZONTAL;
pub const SYM_EMPTY: &str = graph::EMPTY;
pub const SYM_MERGE_LEFT_FROM: &str = graph::MERGE_LEFT_FROM;
pub const SYM_MERGE_RIGHT_FROM: &str = graph::MERGE_RIGHT_FROM;
pub const SYM_BRANCH_UP: &str = graph::BRANCH_UP;
pub const SYM_BRANCH_DOWN: &str = graph::BRANCH_DOWN;
pub const SYM_MERGE: &str = graph::MERGE;
pub const SYM_UNCOMMITED: &str = graph::UNCOMMITTED;
pub const SYM_TAG: &str = entity::TAG;
pub const SYM_CHECKBOX_OFF: &str = form::CHECKBOX_OFF;
pub const SYM_CHECKBOX_ON: &str = form::CHECKBOX_ON;
pub const SYM_RADIO_OFF: &str = form::RADIO_OFF;
pub const SYM_RADIO_ON: &str = form::RADIO_ON;
