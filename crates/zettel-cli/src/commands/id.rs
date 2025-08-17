// crates/zettel-cli/src/commands/id.rs - ID Manipulation Commands
//
// This module implements the core ID manipulation commands that power the
// zettelkasten system. These commands are designed to be composable and
// scriptable - each does one thing well and outputs exactly what it computes.
//
// LUHMANN ID SYSTEM EXPLANATION:
// Niklas Luhmann's ID system creates a branching hierarchy of ideas:
// - 1, 2, 3... (main topics)
// - 1a, 1b, 1c... (subtopics of 1)
// - 1a1, 1a2, 1a3... (sub-subtopics of 1a)
// - 1a1a, 1a1b... (further branching)

use anyhow::Result;
use zettel_core::id::Id;

use crate::cli::IdCommands;
use crate::context::Context;

/// Handle all ID manipulation commands
///
/// This is the main entry point for ID operations. It pattern matches on the
/// specific command variant and delegates to the appropriate logic.
///
/// DESIGN PATTERN: Command Handler
/// Each command variant is handled separately, keeping the logic focused
/// and making it easy to add new ID operations in the future.
///
/// ERROR STRATEGY:
/// - Parse errors are propagated up with context
/// - Business logic errors (like ID overflow) are handled gracefully
/// - User errors result in helpful messages and non-zero exit codes
/// - System errors (like I/O failures) are passed through
///
/// OUTPUT DESIGN:
/// - Success: Print result to stdout (for piping)
/// - Errors: Print to stderr and exit with non-zero code
/// - Verbose info: Only when explicitly requested (future feature)
pub fn handle(ctx: &Context, cmd: IdCommands) -> Result<()> {
    // Get ID manager configured with vault-specific existence checking
    // This provides the core ID manipulation functionality with knowledge
    // of which IDs already exist in this vault
    let id_manager = ctx.get_id_manager();

    match cmd {
        IdCommands::NextSibling { id } => handle_next_sibling(&id_manager, &id),

        IdCommands::NextChild { id } => handle_next_child(&id_manager, &id),

        IdCommands::Parse { filename } => handle_parse(&id_manager, &filename, ctx),

        IdCommands::Validate { id } => handle_validate(&id),
    }
}

/// Generate the next available sibling ID
///
/// Sibling notes exist at the same hierarchical level. Examples:
/// - 1 -> 2 (next root note)
/// - 1a -> 1b (next child of note 1)
/// - 1a2 -> 1a3 (next grandchild of note 1a)
///
/// ALGORITHM:
/// 1. Parse the input ID to validate format
/// 2. Use ID manager to find next available sibling
/// 3. Output the result for use in scripts or manual note creation
///
/// BUSINESS LOGIC:
/// The ID manager handles the complex logic of:
/// - Incrementing the last component of the ID
/// - Checking which IDs already exist in the vault
/// - Finding the first available ID in the sequence
/// - Handling edge cases like alphabet overflow (z -> aa)
///
/// USAGE PATTERNS:
/// ```bash
/// # Interactive use
/// zettel id next-sibling 1a2
/// # Output: 1a3
///
/// # In shell scripts
/// current_id=$(zettel id parse current_note.md)
/// next_id=$(zettel id next-sibling "$current_id")
/// zettel note create "$next_id" "Related Idea"
///
/// # Editor integration
/// next_id=$(zettel id next-sibling 1a)
/// echo "Next sibling would be: $next_id"
/// ```
///
/// ERROR CASES:
/// - Invalid ID format: "abc" (must start with number)
/// - ID overflow: Extremely rare but possible with very deep hierarchies
/// - Parse errors: Malformed ID strings
fn handle_next_sibling<F>(id_manager: &zettel_core::id::IdManager<F>, id: &str) -> Result<()>
where
    F: Fn(&str) -> bool,
{
    // Parse and validate the input ID
    // This catches common user errors like invalid format early
    let current_id = Id::parse(id).map_err(|e| anyhow::anyhow!("Invalid ID '{}': {}", id, e))?;

    // Generate next available sibling using business logic from core
    // The ID manager handles existence checking and conflict resolution
    let next_id = id_manager
        .next_available_sibling(&current_id)
        .map_err(|e| anyhow::anyhow!("Failed to generate sibling ID: {}", e))?;

    // Output result to stdout for piping and scripting
    // Simple format: just the ID, no extra text
    println!("{}", next_id);

    Ok(())
}

/// Generate the next available child ID
///
/// Child notes represent conceptual elaboration or branching from a parent idea.
/// The relationship follows Luhmann's alternating number/letter pattern:
/// - After number comes letter: 1 -> 1a
/// - After letter comes number: 1a -> 1a1
///
/// CONCEPTUAL MODEL:
/// Children represent ideas that develop, elaborate, or branch from the parent.
/// Use children when:
/// - Exploring a sub-aspect of the parent idea
/// - Providing evidence or examples for the parent
/// - Developing a tangent that's related but distinct
///
/// TECHNICAL IMPLEMENTATION:
/// The core ID library handles the alternating pattern logic and ensures
/// the generated ID follows Luhmann's structural rules.
///
/// EXAMPLES:
/// ```bash
/// # Basic child creation
/// zettel id next-child 1
/// # Output: 1a
///
/// # Deeper nesting
/// zettel id next-child 1a
/// # Output: 1a1
///
/// # Complex hierarchy
/// zettel id next-child 1a2b
/// # Output: 1a2b1
/// ```
///
/// WORKFLOW INTEGRATION:
/// This command is typically used when you're reading a note and want to
/// create a child note to explore a sub-idea:
/// ```bash
/// # While editing note "1a2 - Machine Learning Basics"
/// child_id=$(zettel id next-child 1a2)
/// zettel note create "$child_id" "Neural Network Architectures"
/// ```
fn handle_next_child<F>(id_manager: &zettel_core::id::IdManager<F>, id: &str) -> Result<()>
where
    F: Fn(&str) -> bool,
{
    // Parse parent ID and validate format
    let parent_id =
        Id::parse(id).map_err(|e| anyhow::anyhow!("Invalid parent ID '{}': {}", id, e))?;

    // Generate first available child ID
    // The ID manager determines the correct pattern (letter vs number)
    // and finds the first available ID in that sequence
    let child_id = id_manager.next_available_child(&parent_id);

    // Output result for use in note creation workflows
    println!("{}", child_id);

    Ok(())
}

/// Extract ID from filename using vault's matching rules
///
/// This command bridges between the file system (filenames) and the ID system
/// (hierarchical identifiers). It's essential for editor integration where
/// you need to know the ID of the currently open file.
///
/// MATCHING STRATEGIES:
/// The vault configuration determines how IDs are extracted:
/// - Strict: "1a2.md" -> "1a2" (filename is exactly ID + extension)
/// - Separator: "1a2 - Title.md" -> "1a2" (ID + separator + title)
/// - Fuzzy: "1a2_anything.md" -> "1a2" (ID + any non-alphanumeric + anything)
///
/// EDITOR INTEGRATION PATTERN:
/// Most editors can provide the current filename to shell commands:
/// ```bash
/// # Helix editor integration
/// current_id=$(zettel id parse "$CURRENT_FILE")
/// next_sibling=$(zettel id next-sibling "$current_id")
///
/// # Vim integration  
/// :!zettel id parse %
///
/// # VSCode integration (via tasks)
/// zettel id parse "${file}"
/// ```
///
/// ERROR HANDLING:
/// If no valid ID is found, the command exits with error code 1 and
/// provides a helpful message. This prevents scripts from continuing
/// with invalid data.
///
/// TROUBLESHOOTING:
/// If this command fails to parse your filename, check:
/// 1. Vault configuration (match_rule setting)
/// 2. Filename format matches expected pattern
/// 3. ID is valid Luhmann format (starts with number, alternates)
fn handle_parse<F>(
    id_manager: &zettel_core::id::IdManager<F>,
    filename: &str,
    ctx: &Context,
) -> Result<()>
where
    F: Fn(&str) -> bool,
{
    // Extract ID using vault's configured matching rules
    // This might return None if filename doesn't match expected pattern
    if let Some(id) = id_manager.extract_from_filename(filename) {
        // Success: output the parsed ID
        println!("{}", id);
        Ok(())
    } else {
        // Failure: provide helpful error message and exit with error code
        eprintln!("No valid ID found in filename: {}", filename);
        eprintln!("Check vault configuration if this seems wrong.");
        eprintln!("Current match rule: {}", ctx.id_config().match_rule);
        std::process::exit(1);
    }
}

/// Validate ID format and show detailed information
///
/// This command serves multiple purposes:
/// - Validation: Check if an ID string is valid Luhmann format
/// - Education: Show users what makes an ID valid or invalid
/// - Debugging: Help troubleshoot ID generation or parsing issues
/// - Exploration: Understand the structure of complex IDs
///
/// VALIDATION RULES:
/// A valid Luhmann ID must:
/// 1. Start with a number (1, 2, 42, etc.)
/// 2. Alternate between numbers and letters (1a2b3c)
/// 3. Use only lowercase letters (no A, B, C)
/// 4. Contain no spaces or special characters
/// 5. Have at least one component
///
/// OUTPUT FORMAT:
/// For valid IDs, shows:
/// - Confirmation that ID is valid
/// - Hierarchical depth (number of components)
/// - Whether it's a root note (single number)
/// - Parent ID (if not root)
/// - Other structural information
///
/// For invalid IDs, explains:
/// - What rule was violated
/// - Examples of correct format
/// - Suggestions for fixing the ID
///
/// EDUCATIONAL USE:
/// This command helps users understand the ID system:
/// ```bash
/// zettel id validate 1         # Root note
/// zettel id validate 1a        # Child of 1
/// zettel id validate 1a2       # Grandchild
/// zettel id validate 1A2       # Invalid: uppercase
/// zettel id validate abc       # Invalid: no number
/// ```
///
/// DEBUGGING USE:
/// When ID generation seems wrong, validate the IDs to understand structure:
/// ```bash
/// zettel id validate $(zettel id next-sibling 1z)
/// # Shows the structure of the generated sibling
/// ```
fn handle_validate(id: &str) -> Result<()> {
    match Id::parse(id) {
        Ok(parsed_id) => {
            // Valid ID: show detailed structural information
            println!("✅ Valid ID: {}", parsed_id);
            println!("   Components: {}", parsed_id.components().len());
            println!("   Depth: {}", parsed_id.depth());
            println!("   Root note: {}", parsed_id.is_root());

            // Show parent relationship if this isn't a root note
            if let Ok(Some(parent)) = parsed_id.parent() {
                println!("   Parent: {}", parent);
            }

            // Show component breakdown for educational purposes
            print!("   Structure: ");
            for (i, component) in parsed_id.components().iter().enumerate() {
                if i > 0 {
                    print!(" → ");
                }
                match component {
                    zettel_core::id::IdComponent::Numeric(n) => print!("{}(num)", n),
                    zettel_core::id::IdComponent::Alpha(s) => print!("{}(alpha)", s),
                }
            }
            println!();

            // Show what siblings and children would look like
            if let Ok(next_sibling) = parsed_id.next_sibling() {
                println!("   Next sibling would be: {}", next_sibling);
            }
            let first_child = parsed_id.first_child();
            println!("   First child would be: {}", first_child);
        }

        Err(e) => {
            // Invalid ID: explain what's wrong and how to fix it
            eprintln!("❌ Invalid ID: {}", e);
            eprintln!();
            eprintln!("Valid Luhmann ID format:");
            eprintln!("  • Must start with a number: 1, 2, 42");
            eprintln!("  • Numbers and letters alternate: 1a, 1a2, 1a2b");
            eprintln!("  • Only lowercase letters: a, b, c (not A, B, C)");
            eprintln!("  • No spaces or special characters");
            eprintln!();
            eprintln!("Examples:");
            eprintln!("  ✅ 1        (root note)");
            eprintln!("  ✅ 1a       (child of 1)");
            eprintln!("  ✅ 1a2      (grandchild)");
            eprintln!("  ✅ 42z99    (complex ID)");
            eprintln!("  ❌ a1       (starts with letter)");
            eprintln!("  ❌ 1A       (uppercase letter)");
            eprintln!("  ❌ 1-2      (special character)");
            eprintln!("  ❌ 1 a      (contains space)");

            std::process::exit(1);
        }
    }

    Ok(())
}

// COMMAND COMPOSABILITY EXAMPLES:
//
// These commands are designed to work together and with other Unix tools:
//
// ```bash
// # Generate a series of sibling notes
// for i in {1..5}; do
//   id=$(zettel id next-sibling $prev_id)
//   zettel note create "$id" "Topic $i"
//   prev_id="$id"
// done
//
// # Validate all existing note IDs
// zettel list --json | jq -r '.[].id' | while read id; do
//   zettel id validate "$id"
// done
//
// # Find the deepest note in the hierarchy
// zettel list --json | jq -r '.[].id' | while read id; do
//   depth=$(zettel id validate "$id" | grep "Depth:" | cut -d: -f2 | tr -d ' ')
//   echo "$depth $id"
// done | sort -n | tail -1
//
// # Create a child note for the currently open file
// current_id=$(zettel id parse "$CURRENT_FILE")
// child_id=$(zettel id next-child "$current_id")
// zettel note create "$child_id" --open
// ```
//
// DESIGN PRINCIPLES DEMONSTRATED:
//
// 1. SINGLE RESPONSIBILITY:
//    Each command does exactly one ID operation and outputs the result.
//
// 2. COMPOSABILITY:
//    Commands output simple text that can be captured and used by other commands.
//
// 3. SCRIPTABILITY:
//    No interactive prompts or complex output formatting that breaks scripts.
//
// 4. ERROR HANDLING:
//    Clear distinction between success (stdout) and errors (stderr + exit code).
//
// 5. USER EXPERIENCE:
//    Helpful error messages that explain problems and suggest solutions.
//
// 6. PERFORMANCE:
//    Pure computation with minimal file I/O for fast response times.
//
// These patterns make the commands useful both for interactive use and as
// building blocks for more complex automation and editor integration.
