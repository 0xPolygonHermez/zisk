use super::{ProgramRegistry, UNKNOWN_PROGRAM_ALIAS};

const COLLIDING_HASH_A: &str = "7f83b1650f8c4d2a91e6b7304c5a2d8f3b9e61007acdd15ef2438769b5c4a301";
const COLLIDING_HASH_B: &str = "7f83b165c8d41e7a209fb35e6a1c4d908f7ab63215e0dc49a12f87643bd509ef";

fn hash_fixture(seed: &str) -> String {
    blake3::hash(seed.as_bytes()).to_hex().to_string()
}

fn first_chars(value: &str, count: usize) -> String {
    value.chars().take(count).collect()
}

#[test]
fn register_returns_existing_alias_idempotently() {
    let mut registry = ProgramRegistry::with_max(2);
    let hash = hash_fixture("idempotent program registration");
    let expected_alias = first_chars(&hash, 8);

    let first = registry.register(&hash).unwrap();
    let second = registry.register(&hash).unwrap();

    assert_eq!(first, expected_alias);
    assert_eq!(second, first);
}

#[test]
fn register_extends_alias_on_collision() {
    let mut registry = ProgramRegistry::with_max(2);

    let first = registry.register(COLLIDING_HASH_A).unwrap();
    let second = registry.register(COLLIDING_HASH_B).unwrap();

    assert_eq!(first, first_chars(COLLIDING_HASH_A, 8));
    assert_eq!(second, first_chars(COLLIDING_HASH_B, 9));
}

#[test]
fn register_enforces_max_cap_for_new_hashes() {
    let mut registry = ProgramRegistry::with_max(1);
    let registered_hash = hash_fixture("registered program");
    let overflow_hash = hash_fixture("overflow program");
    let expected_alias = first_chars(&registered_hash, 8);

    let first = registry.register(&registered_hash).unwrap();
    let overflow = registry.register(&overflow_hash);
    let repeated = registry.register(&registered_hash).unwrap();

    assert_eq!(first, expected_alias);
    assert_eq!(overflow, None);
    assert_eq!(repeated, first);
}

#[test]
fn alias_uses_first_eight_hash_characters_without_collision() {
    let mut registry = ProgramRegistry::with_max(1);
    let hash = hash_fixture("single program");

    let alias = registry.register(&hash).unwrap();

    assert_eq!(alias, first_chars(&hash, 8));
}

#[test]
fn empty_hash_registers_as_unknown_without_consuming_capacity() {
    let mut registry = ProgramRegistry::with_max(1);
    let hash = hash_fixture("real program");

    assert_eq!(registry.register("").unwrap(), UNKNOWN_PROGRAM_ALIAS);
    assert_eq!(registry.register(&hash).unwrap(), first_chars(&hash, 8));
}
