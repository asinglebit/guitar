use crate::core::chunk::NONE;
use git2::Oid;
use gix::ObjectId;
use iddqd::{IdHashItem, IdHashMap, id_upcast};

pub trait IntoGixOid {
    fn into_gix_oid(self) -> ObjectId;
}

impl IntoGixOid for ObjectId {
    fn into_gix_oid(self) -> ObjectId {
        self
    }
}

impl IntoGixOid for Oid {
    fn into_gix_oid(self) -> ObjectId {
        git2_to_gix_oid(self)
    }
}

pub fn git2_to_gix_oid(oid: Oid) -> gix::ObjectId {
    gix::ObjectId::from_bytes_or_panic(oid.as_bytes())
}

#[cfg(test)]
#[path = "../tests/core/oids.rs"]
mod tests;

pub fn gix_to_git2_oid(oid: gix::ObjectId) -> Oid {
    Oid::from_bytes(oid.as_bytes()).unwrap()
}

// Stores full OIDs once and passes small numeric aliases through UI data structures.
#[derive(Clone)]
pub struct Oids {
    pub zero: ObjectId,
    records: IdHashMap<OidRecord>,
    alias_oids: Vec<ObjectId>,
    pub sorted_aliases: Vec<u32>,
    pub stashes: Vec<u32>,
}

#[derive(Clone, Debug)]
struct OidRecord {
    oid: ObjectId,
    alias: u32,
}

impl IdHashItem for OidRecord {
    type Key<'a> = &'a ObjectId;

    fn key(&self) -> Self::Key<'_> {
        &self.oid
    }

    id_upcast!();
}

impl Default for Oids {
    fn default() -> Self {
        Oids { zero: ObjectId::null(gix::hash::Kind::Sha1), records: IdHashMap::default(), alias_oids: Vec::new(), sorted_aliases: vec![NONE], stashes: vec![] }
    }
}

impl Oids {
    pub fn reserve_total_aliases(&mut self, total: usize) {
        reserve_vec(&mut self.sorted_aliases, total.saturating_add(1));
        reserve_vec(&mut self.alias_oids, total);
    }

    pub fn reserve_aliases(&mut self, additional: usize) {
        self.sorted_aliases.reserve(additional);
        self.alias_oids.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.alias_oids.shrink_to_fit();
        self.sorted_aliases.shrink_to_fit();
        self.stashes.shrink_to_fit();
    }

    pub fn get_alias_by_oid(&mut self, oid: impl IntoGixOid) -> u32 {
        let oid = oid.into_gix_oid();
        if let Some(record) = self.records.get(&oid) {
            return record.alias;
        }

        let alias = u32::try_from(self.alias_oids.len()).expect("OID alias space exhausted");
        self.alias_oids.push(oid);
        self.records.insert_unique(OidRecord { oid, alias }).expect("new OID record has unique OID");

        alias
    }

    pub fn get_existing_alias(&self, oid: impl IntoGixOid) -> Option<u32> {
        let oid = oid.into_gix_oid();
        self.records.get(&oid).map(|record| record.alias)
    }

    pub fn get_alias_by_idx(&self, idx: usize) -> u32 {
        *self.sorted_aliases.get(idx).unwrap()
    }

    pub fn get_oid_by_alias(&self, alias: u32) -> &ObjectId {
        self.alias_oids.get(alias as usize).unwrap_or(&self.zero)
    }

    pub fn get_git2_oid_by_alias(&self, alias: u32) -> Oid {
        gix_to_git2_oid(*self.get_oid_by_alias(alias))
    }

    pub fn get_sorted_aliases(&self) -> &[u32] {
        &self.sorted_aliases
    }

    pub fn append_sorted_alias(&mut self, alias: u32) {
        self.sorted_aliases.push(alias);
    }

    pub fn get_commit_count(&self) -> usize {
        self.sorted_aliases.len()
    }

    pub fn len(&self) -> usize {
        self.alias_oids.len()
    }

    pub fn iter_oids(&self) -> impl Iterator<Item = &ObjectId> {
        self.alias_oids.iter()
    }

    pub fn get_alias_by_prefix(&self, prefix: &str) -> Option<u32> {
        if !is_git_hex_prefix(prefix) {
            return None;
        }

        self.alias_oids.iter().position(|oid| matches_ascii_hex_prefix(oid, prefix)).and_then(|alias| u32::try_from(alias).ok())
    }

    pub fn is_zero(&self, oid: &ObjectId) -> bool {
        self.zero == *oid
    }
}

fn reserve_vec<T>(vec: &mut Vec<T>, target_len: usize) {
    vec.reserve(target_len.saturating_sub(vec.len()));
}

fn matches_ascii_hex_prefix(oid: &ObjectId, prefix: &str) -> bool {
    prefix.bytes().enumerate().all(|(idx, byte)| ascii_hex_nibble(byte).zip(oid_hex_nibble(oid, idx)).is_some_and(|(prefix, oid)| prefix == oid))
}

fn is_git_hex_prefix(prefix: &str) -> bool {
    !prefix.is_empty() && prefix.as_bytes().iter().all(u8::is_ascii_hexdigit)
}

fn oid_hex_nibble(oid: &ObjectId, idx: usize) -> Option<u8> {
    oid.as_bytes().get(idx / 2).map(|byte| if idx % 2 == 0 { byte >> 4 } else { byte & 0x0f })
}

fn ascii_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
