use crate::core::chunk::NONE;
use git2::Oid;
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

pub fn git2_to_gix_oid(oid: Oid) -> gix::ObjectId {
    gix::ObjectId::from_bytes_or_panic(oid.as_bytes())
}

#[cfg(test)]
#[path = "../tests/core/oids.rs"]
mod tests;

pub fn gix_to_git2_oid(oid: gix::ObjectId) -> Oid {
    Oid::from_bytes(oid.as_bytes()).unwrap()
}

pub fn gix_time_to_git2_time(time: gix::date::Time) -> git2::Time {
    debug_assert_eq!(time.offset % 60, 0);
    git2::Time::new(time.seconds, time.offset / 60)
}

// Stores full OIDs once and passes small numeric aliases through UI data structures.
#[derive(Clone)]
pub struct Oids {
    pub zero: Oid,
    pub oids: Vec<Oid>,
    aliases: FxHashMap<u64, u32>,
    alias_collisions: FxHashMap<u64, CollisionBucket>,
    pub sorted_aliases: Vec<u32>,
    pub stashes: Vec<u32>,
}

#[derive(Clone)]
enum CollisionBucket {
    Few(Vec<u32>),
    Many(FxHashMap<Oid, u32>),
}

impl CollisionBucket {
    fn find(&self, oids: &[Oid], oid: Oid) -> Option<u32> {
        match self {
            CollisionBucket::Few(aliases) => aliases.iter().copied().find(|alias| oids.get(*alias as usize).is_some_and(|current| *current == oid)),
            CollisionBucket::Many(aliases) => aliases.get(&oid).copied(),
        }
    }

    fn push(&mut self, oids: &[Oid], oid: Oid, alias: u32) {
        match self {
            CollisionBucket::Few(aliases) if aliases.len() < 8 => aliases.push(alias),
            CollisionBucket::Few(aliases) => {
                let mut exact = FxHashMap::default();
                for existing in aliases.iter().copied() {
                    if let Some(existing_oid) = oids.get(existing as usize) {
                        exact.insert(*existing_oid, existing);
                    }
                }
                exact.insert(oid, alias);
                *self = CollisionBucket::Many(exact);
            },
            CollisionBucket::Many(aliases) => {
                aliases.insert(oid, alias);
            },
        }
    }
}

impl Default for Oids {
    fn default() -> Self {
        Oids { zero: Oid::zero(), oids: Vec::new(), aliases: FxHashMap::default(), alias_collisions: FxHashMap::default(), sorted_aliases: vec![NONE], stashes: vec![] }
    }
}

impl Oids {
    pub fn reserve_aliases(&mut self, additional: usize) {
        self.oids.reserve(additional);
        self.aliases.reserve(additional);
    }

    pub fn get_alias_by_oid(&mut self, oid: Oid) -> u32 {
        // Assign aliases lazily so refs, commits, tags, and stashes share one namespace.
        let fingerprint = oid_fingerprint(oid);
        if let Some(&alias) = self.aliases.get(&fingerprint) {
            if self.oids.get(alias as usize).is_some_and(|current| *current == oid) {
                return alias;
            }
            if let Some(alias) = self.alias_collisions.get(&fingerprint).and_then(|bucket| bucket.find(&self.oids, oid)) {
                return alias;
            }
        }

        let alias = self.oids.len() as u32;
        self.oids.push(oid);

        match self.aliases.entry(fingerprint) {
            Entry::Occupied(_) => match self.alias_collisions.entry(fingerprint) {
                Entry::Occupied(mut entry) => entry.get_mut().push(&self.oids, oid, alias),
                Entry::Vacant(entry) => {
                    entry.insert(CollisionBucket::Few(vec![alias]));
                },
            },
            Entry::Vacant(entry) => {
                entry.insert(alias);
            },
        }

        alias
    }

    pub fn get_existing_alias(&self, oid: Oid) -> Option<u32> {
        let fingerprint = oid_fingerprint(oid);
        let alias = *self.aliases.get(&fingerprint)?;
        if self.oids.get(alias as usize).is_some_and(|current| *current == oid) {
            return Some(alias);
        }
        self.alias_collisions.get(&fingerprint).and_then(|bucket| bucket.find(&self.oids, oid))
    }

    pub fn get_alias_by_idx(&self, idx: usize) -> u32 {
        *self.sorted_aliases.get(idx).unwrap()
    }

    pub fn get_oid_by_alias(&self, alias: u32) -> &Oid {
        self.oids.get(alias as usize).unwrap_or(&self.zero)
    }

    pub fn get_oid_by_idx(&self, idx: usize) -> &Oid {
        let alias = *self.sorted_aliases.get(idx).unwrap_or(&NONE);
        self.oids.get(alias as usize).unwrap_or(&self.zero)
    }

    pub fn get_sorted_aliases(&self) -> &Vec<u32> {
        &self.sorted_aliases
    }

    pub fn append_sorted_alias(&mut self, alias: u32) {
        self.sorted_aliases.push(alias);
    }

    pub fn get_commit_count(&self) -> usize {
        self.sorted_aliases.len()
    }

    pub fn is_zero(&self, oid: &Oid) -> bool {
        self.zero == *oid
    }
}

fn oid_fingerprint(oid: Oid) -> u64 {
    let bytes = oid.as_bytes();
    u64::from_be_bytes(bytes[..8].try_into().unwrap())
}
