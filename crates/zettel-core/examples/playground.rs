// crates/zettel-core/examples/playground.rs
// Run with: cargo run --example playground

use zettel_core::id::*;

fn main() {
    println!("🗂️ Zettel ID System Playground\n");

    // Test basic ID creation
    println!("=== Basic ID Creation ===");
    let id1 = Id::parse("1").unwrap();
    let id2 = Id::parse("1a").unwrap();
    let id3 = Id::parse("1a2b3c").unwrap();

    println!("Created IDs: {}, {}, {}", id1, id2, id3);
    println!("Depths: {}, {}, {}", id1.depth(), id2.depth(), id3.depth());
    println!(
        "Root checks: {}, {}, {}",
        id1.is_root(),
        id2.is_root(),
        id3.is_root()
    );

    // Test relationships
    println!("\n=== Relationship Tests ===");
    println!(
        "{} is ancestor of {}? {}",
        id1,
        id2,
        id1.is_ancestor_of(&id2)
    );
    println!(
        "{} is ancestor of {}? {}",
        id1,
        id3,
        id1.is_ancestor_of(&id3)
    );
    println!(
        "{} is descendant of {}? {}",
        id3,
        id1,
        id3.is_descendant_of(&id1)
    );

    // Test siblings
    let sibling1 = Id::parse("1a2b3d").unwrap();
    println!(
        "{} is sibling of {}? {}",
        id3,
        sibling1,
        id3.is_sibling_of(&sibling1)
    );

    // Test ID generation
    println!("\n=== ID Generation ===");
    let current = Id::parse("1a2").unwrap();
    println!("Current ID: {}", current);

    let next_sibling = current.next_sibling().unwrap();
    println!("Next sibling: {}", next_sibling);

    let first_child = current.first_child();
    println!("First child: {}", first_child);

    let child_of_child = first_child.first_child();
    println!("Child of child: {}", child_of_child);

    // Test sequence generation
    println!("\n=== Sequence Generation ===");
    let mut current = Id::parse("1").unwrap();
    print!("Sibling sequence: {}", current);

    for _ in 0..5 {
        current = current.next_sibling().unwrap();
        print!(" → {}", current);
    }
    println!();

    // Test alphabetic sequences
    println!("\n=== Alphabetic Sequences ===");
    let mut alpha_id = Id::parse("1z").unwrap();
    print!("Alpha sequence: {}", alpha_id);

    for _ in 0..5 {
        alpha_id = alpha_id.next_sibling().unwrap();
        print!(" → {}", alpha_id);
    }
    println!();

    // Test filename parsing with different rules
    println!("\n=== Filename Parsing ===");
    test_filename_parsing();

    // Test ID manager
    println!("\n=== ID Manager ===");
    test_id_manager();

    println!("\n✅ All tests completed!");
}

fn test_filename_parsing() {
    let filenames = vec![
        "1a2",
        "1a2.md",
        "1a2 - My Note.md",
        "1a2_note.md",
        "1a2-title.md",
        "not_an_id.md",
    ];

    // Test strict matching
    let strict_config = IdConfig {
        match_rule: "strict".to_string(),
        separator: " - ".to_string(),
        allow_unicode: false,
    };
    let strict_manager = IdManager::new(strict_config, |_| false);

    println!("Strict matching:");
    for filename in &filenames {
        let result = strict_manager.extract_from_filename(filename);
        println!("  {} → {:?}", filename, result.map(|id| id.to_string()));
    }

    // Test separator matching
    let sep_config = IdConfig {
        match_rule: "separator".to_string(),
        separator: " - ".to_string(),
        allow_unicode: false,
    };
    let sep_manager = IdManager::new(sep_config, |_| false);

    println!("Separator matching:");
    for filename in &filenames {
        let result = sep_manager.extract_from_filename(filename);
        println!("  {} → {:?}", filename, result.map(|id| id.to_string()));
    }

    // Test fuzzy matching
    let fuzzy_config = IdConfig {
        match_rule: "fuzzy".to_string(),
        separator: "".to_string(),
        allow_unicode: false,
    };
    let fuzzy_manager = IdManager::new(fuzzy_config, |_| false);

    println!("Fuzzy matching:");
    for filename in &filenames {
        let result = fuzzy_manager.extract_from_filename(filename);
        println!("  {} → {:?}", filename, result.map(|id| id.to_string()));
    }
}

fn test_id_manager() {
    use std::collections::HashSet;

    // Simulate a vault with some existing IDs
    let mut existing_ids = HashSet::new();
    existing_ids.insert("1".to_string());
    existing_ids.insert("2".to_string());
    existing_ids.insert("1a".to_string());
    existing_ids.insert("1b".to_string());
    existing_ids.insert("1a1".to_string());

    let config = IdConfig::default();
    let manager = IdManager::new(config, |id: &str| existing_ids.contains(id));

    println!("Existing IDs: {:?}", existing_ids);

    // Test next available sibling
    let current = Id::parse("1").unwrap();
    let next_sibling = manager.next_available_sibling(&current).unwrap();
    println!("Next available sibling of {}: {}", current, next_sibling);

    let current = Id::parse("2").unwrap();
    let next_sibling = manager.next_available_sibling(&current).unwrap();
    println!("Next available sibling of {}: {}", current, next_sibling);

    // Test next available child
    let current = Id::parse("1").unwrap();
    let next_child = manager.next_available_child(&current);
    println!("Next available child of {}: {}", current, next_child);

    let current = Id::parse("1a").unwrap();
    let next_child = manager.next_available_child(&current);
    println!("Next available child of {}: {}", current, next_child);
}
