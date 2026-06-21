use crate::core::chunk::NONE;
use git2::Oid;
use std::collections::HashMap;

pub fn git2_to_gix_oid(oid: Oid) -> gix::ObjectId {
    gix::ObjectId::from_bytes_or_panic(oid.as_bytes())
}

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
    pub aliases: HashMap<Oid, u32>,
    pub sorted_aliases: Vec<u32>,
    pub stashes: Vec<u32>,
}

impl Default for Oids {
    fn default() -> Self {
        Oids { zero: Oid::zero(), oids: Vec::new(), aliases: HashMap::new(), sorted_aliases: vec![NONE], stashes: vec![] }
    }
}

impl Oids {
    pub fn reserve_aliases(&mut self, additional: usize) {
        self.oids.reserve(additional);
        self.aliases.reserve(additional);
    }

    pub fn get_alias_by_oid(&mut self, oid: Oid) -> u32 {
        // Assign aliases lazily so refs, commits, tags, and stashes share one namespace.
        *self.aliases.entry(oid).or_insert_with(|| {
            self.oids.push(oid);
            self.oids.len() as u32 - 1
        })
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
