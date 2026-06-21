use super::*;

fn oid_with_prefix(prefix: u64, suffix: u64) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[..8].copy_from_slice(&prefix.to_be_bytes());
    bytes[8..16].copy_from_slice(&suffix.to_be_bytes());
    bytes[16..20].copy_from_slice(&(suffix as u32).wrapping_mul(2_654_435_761).to_be_bytes());
    Oid::from_bytes(&bytes).unwrap()
}

#[test]
fn aliases_are_stable_for_repeated_oid() {
    let mut oids = Oids::default();
    let oid = oid_with_prefix(1, 10);

    let first = oids.get_alias_by_oid(oid);
    let second = oids.get_alias_by_oid(oid);

    assert_eq!(first, second);
    assert_eq!(oids.get_oid_by_alias(first), &oid);
    assert_eq!(oids.oids.len(), 1);
}

#[test]
fn aliases_keep_distinct_oids_with_shared_prefix() {
    let mut oids = Oids::default();
    let first_oid = oid_with_prefix(1, 10);
    let second_oid = oid_with_prefix(1, 20);

    let first = oids.get_alias_by_oid(first_oid);
    let second = oids.get_alias_by_oid(second_oid);

    assert_ne!(first, second);
    assert_eq!(oids.get_alias_by_oid(first_oid), first);
    assert_eq!(oids.get_alias_by_oid(second_oid), second);
    assert_eq!(oids.get_oid_by_alias(first), &first_oid);
    assert_eq!(oids.get_oid_by_alias(second), &second_oid);
}

#[test]
fn aliases_with_unique_fingerprints_do_not_allocate_collision_buckets() {
    let mut oids = Oids::default();

    for prefix in 1..=16 {
        oids.get_alias_by_oid(oid_with_prefix(prefix, 10));
    }

    assert_eq!(oids.oids.len(), 16);
    assert!(oids.alias_collisions.is_empty());
}

#[test]
fn missing_alias_lookup_returns_none() {
    let mut oids = Oids::default();
    let present = oid_with_prefix(1, 10);
    let missing = oid_with_prefix(1, 20);

    oids.get_alias_by_oid(present);

    assert_eq!(oids.get_existing_alias(missing), None);
}
