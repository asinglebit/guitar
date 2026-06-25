use git2::Oid;
use ratatui::style::Color;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadReflogAliasEntry {
    pub selector: String,
    pub old_oid: Oid,
    pub new_oid: Oid,
    pub new_alias: u32,
    pub message: String,
    pub time: gix::date::Time,
}

#[derive(Default)]
pub struct HeadReflogs {
    pub entries: Vec<HeadReflogAliasEntry>,
    pub latest_by_alias: HashMap<u32, HeadReflogAliasEntry>,
    pub colors: HashMap<u32, Color>,
}

impl HeadReflogs {
    pub fn latest_for_alias(&self, alias: u32) -> Option<&HeadReflogAliasEntry> {
        self.latest_by_alias.get(&alias)
    }

    pub fn get_color(&self, alias: u32) -> Option<Color> {
        self.colors.get(&alias).copied()
    }
}

#[cfg(test)]
#[path = "../tests/core/reflogs.rs"]
mod tests;
