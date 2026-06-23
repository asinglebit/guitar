use crate::core::chunk::NONE;
use git2::Oid;
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

type OidFingerprint = u32;
const OID_CHUNK_SIZE: usize = 32_768;

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
    pub oids: OidStore,
    aliases: AliasIndex,
    alias_collisions: FxHashMap<OidFingerprint, CollisionBucket>,
    pub sorted_aliases: Vec<u32>,
    pub stashes: Vec<u32>,
}

#[derive(Clone, Default)]
pub struct OidStore {
    chunks: Vec<Vec<Oid>>,
    len: usize,
}

impl OidStore {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn capacity(&self) -> usize {
        self.chunks.iter().map(Vec::capacity).sum()
    }

    fn reserve(&mut self, additional: usize) {
        let needed = self.len.saturating_add(additional);
        let needed_chunks = needed.div_ceil(OID_CHUNK_SIZE);
        if needed_chunks > self.chunks.capacity() {
            self.chunks.reserve(needed_chunks - self.chunks.capacity());
        }
    }

    fn push(&mut self, oid: Oid) {
        if self.chunks.last().is_none_or(|chunk| chunk.len() == OID_CHUNK_SIZE) {
            self.chunks.push(Vec::with_capacity(OID_CHUNK_SIZE));
        }
        self.chunks.last_mut().expect("oid store has a writable chunk").push(oid);
        self.len += 1;
    }

    fn get(&self, idx: usize) -> Option<&Oid> {
        if idx >= self.len {
            return None;
        }
        let chunk = idx / OID_CHUNK_SIZE;
        let offset = idx % OID_CHUNK_SIZE;
        self.chunks.get(chunk).and_then(|chunk| chunk.get(offset))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Oid> {
        self.chunks.iter().flat_map(|chunk| chunk.iter())
    }

    fn shrink_to_fit(&mut self) {
        for chunk in &mut self.chunks {
            chunk.shrink_to_fit();
        }
        self.chunks.shrink_to_fit();
    }
}

#[derive(Clone)]
enum AliasIndex {
    Hash(FxHashMap<OidFingerprint, u32>),
    Flat(Vec<(OidFingerprint, u32)>),
}

impl Default for AliasIndex {
    fn default() -> Self {
        Self::Hash(FxHashMap::default())
    }
}

impl AliasIndex {
    #[cfg(test)]
    fn capacity(&self) -> usize {
        match self {
            AliasIndex::Hash(aliases) => aliases.capacity(),
            AliasIndex::Flat(aliases) => aliases.capacity(),
        }
    }

    fn get(&self, fingerprint: OidFingerprint) -> Option<u32> {
        match self {
            AliasIndex::Hash(aliases) => aliases.get(&fingerprint).copied(),
            AliasIndex::Flat(aliases) => aliases.binary_search_by_key(&fingerprint, |(fingerprint, _)| *fingerprint).ok().map(|idx| aliases[idx].1),
        }
    }

    fn shrink_to_fit(&mut self) {
        match self {
            AliasIndex::Hash(aliases) => aliases.shrink_to_fit(),
            AliasIndex::Flat(aliases) => aliases.shrink_to_fit(),
        }
    }

    fn compact(&mut self) {
        if let AliasIndex::Hash(aliases) = self {
            let mut flat: Vec<_> = aliases.iter().map(|(&fingerprint, &alias)| (fingerprint, alias)).collect();
            flat.sort_unstable_by_key(|(fingerprint, _)| *fingerprint);
            *self = AliasIndex::Flat(flat);
        }
    }

    fn ensure_hash(&mut self) -> &mut FxHashMap<OidFingerprint, u32> {
        if let AliasIndex::Flat(flat) = self {
            let aliases: FxHashMap<OidFingerprint, u32> = flat.iter().copied().collect();
            *self = AliasIndex::Hash(aliases);
        }

        let AliasIndex::Hash(aliases) = self else {
            unreachable!("alias index is materialized as a hash map");
        };
        aliases
    }

    #[cfg(test)]
    fn is_flat(&self) -> bool {
        matches!(self, AliasIndex::Flat(_))
    }
}

#[derive(Clone)]
enum CollisionBucket {
    Few(Vec<u32>),
    Many(FxHashMap<Oid, u32>),
}

impl CollisionBucket {
    fn find(&self, oids: &OidStore, oid: Oid) -> Option<u32> {
        match self {
            CollisionBucket::Few(aliases) => aliases.iter().copied().find(|alias| oids.get(*alias as usize).is_some_and(|current| *current == oid)),
            CollisionBucket::Many(aliases) => aliases.get(&oid).copied(),
        }
    }

    fn push(&mut self, oids: &OidStore, oid: Oid, alias: u32) {
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

    fn shrink_to_fit(&mut self) {
        match self {
            CollisionBucket::Few(aliases) => aliases.shrink_to_fit(),
            CollisionBucket::Many(aliases) => aliases.shrink_to_fit(),
        }
    }
}

impl Default for Oids {
    fn default() -> Self {
        Oids { zero: Oid::zero(), oids: OidStore::default(), aliases: AliasIndex::default(), alias_collisions: FxHashMap::default(), sorted_aliases: vec![NONE], stashes: vec![] }
    }
}

impl Oids {
    pub fn reserve_aliases(&mut self, additional: usize) {
        let oid_spare = self.oids.capacity().saturating_sub(self.oids.len());
        if additional > oid_spare {
            self.oids.reserve(additional - oid_spare);
        }

        // The hash index is compacted after the walk; preallocating it at Linux scale
        // creates a large transient peak for little benefit.
    }

    pub fn compact_alias_index(&mut self) {
        self.aliases.compact();
    }

    pub fn shrink_to_fit(&mut self) {
        self.oids.shrink_to_fit();
        self.aliases.shrink_to_fit();
        self.alias_collisions.shrink_to_fit();
        for bucket in self.alias_collisions.values_mut() {
            bucket.shrink_to_fit();
        }
        self.sorted_aliases.shrink_to_fit();
        self.stashes.shrink_to_fit();
    }

    pub fn get_alias_by_oid(&mut self, oid: Oid) -> u32 {
        // Assign aliases lazily so refs, commits, tags, and stashes share one namespace.
        let fingerprint = oid_fingerprint(oid);
        if let Some(alias) = self.aliases.get(fingerprint) {
            if self.oids.get(alias as usize).is_some_and(|current| *current == oid) {
                return alias;
            }
            if let Some(alias) = self.alias_collisions.get(&fingerprint).and_then(|bucket| bucket.find(&self.oids, oid)) {
                return alias;
            }
        }

        let alias = self.oids.len() as u32;
        self.oids.push(oid);

        match self.aliases.ensure_hash().entry(fingerprint) {
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
        let alias = self.aliases.get(fingerprint)?;
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

fn oid_fingerprint(oid: Oid) -> OidFingerprint {
    let bytes = oid.as_bytes();
    u32::from_be_bytes(bytes[..4].try_into().unwrap())
}
