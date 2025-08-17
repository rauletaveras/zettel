// crates/zettel-core/src/id.rs - Core ID manipulation for Luhmann-style Zettelkasten

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur during ID operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum IdError {
    #[error("Invalid ID format: {0}")]
    InvalidFormat(String),

    #[error("Empty ID not allowed")]
    EmptyId,

    #[error("Invalid component: {0}")]
    InvalidComponent(String),

    #[error("ID overflow: cannot increment {0}")]
    Overflow(String),

    #[error("No parent for root ID: {0}")]
    NoParent(String),

    #[error("Parsing error: {0}")]
    ParseError(String),
}

/// Result type for ID operations
pub type IdResult<T> = Result<T, IdError>;

/// Represents a single component of an ID (either numeric or alphabetic)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IdComponent {
    /// Numeric component (e.g., "1", "42", "123")
    Numeric(u32),
    /// Alphabetic component (e.g., "a", "z", "aa", "abc")
    Alpha(String),
}

impl IdComponent {
    /// Create a new numeric component
    pub fn numeric(value: u32) -> Self {
        Self::Numeric(value)
    }

    /// Create a new alphabetic component
    pub fn alpha<S: Into<String>>(value: S) -> IdResult<Self> {
        let value = value.into();
        if value.is_empty() {
            return Err(IdError::InvalidComponent(
                "Empty alphabetic component".to_string(),
            ));
        }

        if !value.chars().all(|c| c.is_ascii_lowercase()) {
            return Err(IdError::InvalidComponent(format!(
                "Alphabetic component must contain only lowercase letters: {}",
                value
            )));
        }

        Ok(Self::Alpha(value))
    }

    /// Get the next component in sequence
    pub fn increment(&self) -> IdResult<Self> {
        match self {
            Self::Numeric(n) => {
                if *n == u32::MAX {
                    Err(IdError::Overflow(format!("Numeric component: {}", n)))
                } else {
                    Ok(Self::Numeric(n + 1))
                }
            }
            Self::Alpha(s) => {
                let incremented = increment_alpha_string(s)?;
                Ok(Self::Alpha(incremented))
            }
        }
    }

    /// Check if this is a numeric component
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Numeric(_))
    }

    /// Check if this is an alphabetic component
    pub fn is_alpha(&self) -> bool {
        matches!(self, Self::Alpha(_))
    }

    /// Get the value as a string
    pub fn as_str(&self) -> String {
        match self {
            Self::Numeric(n) => n.to_string(),
            Self::Alpha(s) => s.clone(),
        }
    }
}

impl fmt::Display for IdComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric(n) => write!(f, "{}", n),
            Self::Alpha(s) => write!(f, "{}", s),
        }
    }
}

impl FromStr for IdComponent {
    type Err = IdError;

    fn from_str(s: &str) -> IdResult<Self> {
        if s.is_empty() {
            return Err(IdError::InvalidComponent("Empty component".to_string()));
        }

        // Try parsing as number first
        if let Ok(num) = s.parse::<u32>() {
            return Ok(Self::Numeric(num));
        }

        // Otherwise treat as alphabetic
        Self::alpha(s)
    }
}

/// Represents a complete Luhmann-style ID (e.g., "1", "1a", "1a2b", "42c17z")
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Id {
    components: Vec<IdComponent>,
}

impl Id {
    /// Create a new ID from components
    pub fn new(components: Vec<IdComponent>) -> IdResult<Self> {
        if components.is_empty() {
            return Err(IdError::EmptyId);
        }

        // Validate Luhmann alternating pattern: number -> letter -> number -> letter...
        for (i, component) in components.iter().enumerate() {
            let should_be_numeric = i % 2 == 0; // Even indices should be numeric

            if should_be_numeric && !component.is_numeric() {
                return Err(IdError::InvalidFormat(format!(
                    "Component {} should be numeric but got: {}",
                    i, component
                )));
            }

            if !should_be_numeric && !component.is_alpha() {
                return Err(IdError::InvalidFormat(format!(
                    "Component {} should be alphabetic but got: {}",
                    i, component
                )));
            }
        }

        Ok(Self { components })
    }

    /// Create ID from a single numeric component (e.g., "1", "42")
    pub fn from_number(n: u32) -> Self {
        Self {
            components: vec![IdComponent::Numeric(n)],
        }
    }

    /// Parse ID from string (e.g., "1a2b3")
    pub fn parse<S: AsRef<str>>(s: S) -> IdResult<Self> {
        let s = s.as_ref();
        if s.is_empty() {
            return Err(IdError::EmptyId);
        }

        let components = parse_id_string(s)?;
        Self::new(components)
    }

    /// Get all components
    pub fn components(&self) -> &[IdComponent] {
        &self.components
    }

    /// Get the depth/level of this ID (number of components)
    pub fn depth(&self) -> usize {
        self.components.len()
    }

    /// Check if this is a root ID (single numeric component)
    pub fn is_root(&self) -> bool {
        self.components.len() == 1 && self.components[0].is_numeric()
    }

    /// Get the parent ID by removing the last component
    pub fn parent(&self) -> IdResult<Option<Self>> {
        if self.components.len() <= 1 {
            return Ok(None); // Root has no parent
        }

        let mut parent_components = self.components.clone();
        parent_components.pop();

        Ok(Some(Self {
            components: parent_components,
        }))
    }

    /// Get the next sibling ID (increment last component)
    pub fn next_sibling(&self) -> IdResult<Self> {
        if self.components.is_empty() {
            return Err(IdError::EmptyId);
        }

        let mut sibling_components = self.components.clone();
        let last_idx = sibling_components.len() - 1;
        sibling_components[last_idx] = sibling_components[last_idx].increment()?;

        Ok(Self {
            components: sibling_components,
        })
    }

    /// Get the first child ID (append appropriate component type)
    pub fn first_child(&self) -> Self {
        let mut child_components = self.components.clone();

        // Luhmann pattern: numeric -> alpha -> numeric -> alpha...
        let next_component = if self.components.len() % 2 == 0 {
            // Even length means last is alpha, next should be numeric
            IdComponent::Numeric(1)
        } else {
            // Odd length means last is numeric, next should be alpha
            IdComponent::Alpha("a".to_string())
        };

        child_components.push(next_component);
        Self {
            components: child_components,
        }
    }

    /// Check if this ID is an ancestor of another ID
    pub fn is_ancestor_of(&self, other: &Id) -> bool {
        if self.components.len() >= other.components.len() {
            return false; // Can't be ancestor if same length or longer
        }

        // Check if our components are a prefix of the other's components
        self.components
            .iter()
            .zip(other.components.iter())
            .all(|(a, b)| a == b)
    }

    /// Check if this ID is a descendant of another ID
    pub fn is_descendant_of(&self, other: &Id) -> bool {
        other.is_ancestor_of(self)
    }

    /// Check if this ID is a sibling of another ID (same parent)
    pub fn is_sibling_of(&self, other: &Id) -> bool {
        if self.components.len() != other.components.len() {
            return false; // Different depths can't be siblings
        }

        if self.components.len() <= 1 {
            return true; // All roots are siblings
        }

        // Check if all components except the last are the same
        self.components[..self.components.len() - 1]
            .iter()
            .zip(other.components[..other.components.len() - 1].iter())
            .all(|(a, b)| a == b)
    }

    /// Get all ancestor IDs from root to direct parent
    pub fn ancestors(&self) -> Vec<Id> {
        let mut ancestors = Vec::new();

        for i in 1..self.components.len() {
            let ancestor_components = self.components[..i].to_vec();
            ancestors.push(Id {
                components: ancestor_components,
            });
        }

        ancestors
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        self.components
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join("")
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for Id {
    type Err = IdError;

    fn from_str(s: &str) -> IdResult<Self> {
        Self::parse(s)
    }
}

/// ID Manager handles ID generation, validation, and operations within a vault context
pub struct IdManager<F> {
    config: IdConfig,
    existence_checker: F,
}

/// Configuration for ID parsing and generation
///
/// This is now a re-export of the config system's IdConfig to maintain
/// API compatibility while centralizing configuration management.
pub use crate::config::IdConfig;

impl<F> IdManager<F>
where
    F: Fn(&str) -> bool, // Function to check if an ID exists
{
    /// Create new ID manager with configuration and existence checker
    pub fn new(config: IdConfig, existence_checker: F) -> Self {
        Self {
            config,
            existence_checker,
        }
    }

    /// Extract ID from filename based on configured rules
    pub fn extract_from_filename(&self, filename: &str) -> Option<Id> {
        let patterns = self.get_filename_patterns();

        for pattern in patterns {
            if let Some(captures) = pattern.captures(filename) {
                if let Some(id_match) = captures.get(1) {
                    if let Ok(id) = Id::parse(id_match.as_str()) {
                        return Some(id);
                    }
                }
            }
        }

        None
    }

    /// Generate next available sibling ID
    pub fn next_available_sibling(&self, current_id: &Id) -> IdResult<Id> {
        let mut candidate = current_id.next_sibling()?;

        // Keep incrementing until we find an available ID
        while (self.existence_checker)(&candidate.to_string()) {
            candidate = candidate.next_sibling()?;
        }

        Ok(candidate)
    }

    /// Generate next available child ID
    pub fn next_available_child(&self, parent_id: &Id) -> Id {
        let mut candidate = parent_id.first_child();

        // Keep incrementing until we find an available ID
        while (self.existence_checker)(&candidate.to_string()) {
            // For children, we increment the last component
            if let Ok(next) = candidate.next_sibling() {
                candidate = next;
            } else {
                // If we can't increment, something's wrong - return the first child
                break;
            }
        }

        candidate
    }

    /// Validate an ID string
    pub fn validate_id(&self, id_str: &str) -> IdResult<Id> {
        Id::parse(id_str)
    }

    /// Check if an ID exists in the vault
    pub fn id_exists(&self, id: &Id) -> bool {
        (self.existence_checker)(&id.to_string())
    }

    /// Get filename patterns based on configuration
    fn get_filename_patterns(&self) -> Vec<Regex> {
        let id_pattern = if self.config.allow_unicode {
            r"([0-9\p{L}]+(?:[0-9\p{L}]*)*)"
        } else {
            r"([0-9a-z]+(?:[0-9a-z]*)*)"
        };

        let mut patterns = Vec::new();

        match self.config.match_rule.as_str() {
            "strict" => {
                // Filename is exactly the ID: "1a2.md"
                patterns.push(Regex::new(&format!(r"^{}$", id_pattern)).unwrap());
            }
            "separator" => {
                // ID followed by separator: "1a2 - Title.md"
                let escaped_sep = regex::escape(&self.config.separator);
                patterns.push(Regex::new(&format!(r"^{}{}.*", id_pattern, escaped_sep)).unwrap());
            }
            "fuzzy" => {
                // ID at start, anything after first non-alphanumeric: "1a2_title.md", "1a2-title.md"
                patterns.push(Regex::new(&format!(r"^{}[^0-9a-z].*", id_pattern)).unwrap());
                // Also match strict format as fallback
                patterns.push(Regex::new(&format!(r"^{}$", id_pattern)).unwrap());
            }
            _ => {
                // Default to strict
                patterns.push(Regex::new(&format!(r"^{}$", id_pattern)).unwrap());
            }
        }

        patterns
    }
}

/// Parse ID string into components
fn parse_id_string(s: &str) -> IdResult<Vec<IdComponent>> {
    if s.is_empty() {
        return Err(IdError::EmptyId);
    }

    let mut components = Vec::new();
    let mut current = String::new();
    let mut expecting_numeric = true; // Luhmann IDs start with numbers

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            if !expecting_numeric && !current.is_empty() {
                // We were building an alpha component, finish it
                components.push(IdComponent::alpha(current.clone())?);
                current.clear();
                expecting_numeric = true;
            }
            current.push(ch);
        } else if ch.is_ascii_lowercase() {
            if expecting_numeric && !current.is_empty() {
                // We were building a numeric component, finish it
                let num: u32 = current
                    .parse()
                    .map_err(|_| IdError::ParseError(format!("Invalid number: {}", current)))?;
                components.push(IdComponent::Numeric(num));
                current.clear();
                expecting_numeric = false;
            }
            current.push(ch);
        } else {
            return Err(IdError::InvalidFormat(format!(
                "Invalid character '{}' in ID: {}",
                ch, s
            )));
        }
    }

    // Handle the last component
    if !current.is_empty() {
        if expecting_numeric {
            let num: u32 = current
                .parse()
                .map_err(|_| IdError::ParseError(format!("Invalid number: {}", current)))?;
            components.push(IdComponent::Numeric(num));
        } else {
            components.push(IdComponent::alpha(current)?);
        }
    }

    if components.is_empty() {
        return Err(IdError::EmptyId);
    }

    Ok(components)
}

/// Increment an alphabetic string (a -> b, z -> aa, az -> ba)
fn increment_alpha_string(s: &str) -> IdResult<String> {
    if s.is_empty() {
        return Err(IdError::InvalidComponent(
            "Empty alphabetic string".to_string(),
        ));
    }

    let mut chars: Vec<char> = s.chars().collect();
    let mut carry = true;

    // Process from right to left (like adding 1 to a number)
    for i in (0..chars.len()).rev() {
        if !carry {
            break;
        }

        if chars[i] == 'z' {
            chars[i] = 'a';
            // carry remains true
        } else {
            chars[i] = (chars[i] as u8 + 1) as char;
            carry = false;
        }
    }

    // If we still have carry, we need to add a new 'a' at the beginning
    if carry {
        chars.insert(0, 'a');
    }

    Ok(chars.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_component_creation() {
        let num = IdComponent::numeric(42);
        assert_eq!(num.as_str(), "42");
        assert!(num.is_numeric());
        assert!(!num.is_alpha());

        let alpha = IdComponent::alpha("abc").unwrap();
        assert_eq!(alpha.as_str(), "abc");
        assert!(alpha.is_alpha());
        assert!(!alpha.is_numeric());

        // Test invalid alpha component
        assert!(IdComponent::alpha("").is_err());
        assert!(IdComponent::alpha("ABC").is_err());
        assert!(IdComponent::alpha("a1b").is_err());
        assert!(IdComponent::alpha("123").is_err());
    }

    #[test]
    fn test_component_increment() {
        // Numeric increment
        let num = IdComponent::numeric(5);
        let next = num.increment().unwrap();
        assert_eq!(next, IdComponent::numeric(6));

        // Test numeric overflow
        let max_num = IdComponent::numeric(u32::MAX);
        assert!(max_num.increment().is_err());

        // Alpha increment
        let alpha = IdComponent::alpha("a").unwrap();
        let next = alpha.increment().unwrap();
        assert_eq!(next, IdComponent::alpha("b").unwrap());

        let z = IdComponent::alpha("z").unwrap();
        let next = z.increment().unwrap();
        assert_eq!(next, IdComponent::alpha("aa").unwrap());

        let az = IdComponent::alpha("az").unwrap();
        let next = az.increment().unwrap();
        assert_eq!(next, IdComponent::alpha("ba").unwrap());
    }

    #[test]
    fn test_alpha_string_increment() {
        assert_eq!(increment_alpha_string("a").unwrap(), "b");
        assert_eq!(increment_alpha_string("z").unwrap(), "aa");
        assert_eq!(increment_alpha_string("az").unwrap(), "ba");
        assert_eq!(increment_alpha_string("zz").unwrap(), "aaa");
        assert_eq!(increment_alpha_string("abc").unwrap(), "abd");
        assert_eq!(increment_alpha_string("abz").unwrap(), "aca");

        // Test empty string error
        assert!(increment_alpha_string("").is_err());
    }

    #[test]
    fn test_id_parsing() {
        // Simple cases
        let id = Id::parse("1").unwrap();
        assert_eq!(id.components().len(), 1);
        assert!(id.is_root());
        assert_eq!(id.depth(), 1);

        let id = Id::parse("1a").unwrap();
        assert_eq!(id.components().len(), 2);
        assert!(!id.is_root());
        assert_eq!(id.depth(), 2);

        let id = Id::parse("1a2b3c").unwrap();
        assert_eq!(id.components().len(), 6);
        assert_eq!(id.depth(), 6);

        // Complex IDs
        let id = Id::parse("42z123a5").unwrap();
        assert_eq!(id.components().len(), 4);
        assert_eq!(id.components()[0], IdComponent::numeric(42));
        assert_eq!(id.components()[1], IdComponent::alpha("z").unwrap());
        assert_eq!(id.components()[2], IdComponent::numeric(123));
        assert_eq!(id.components()[3], IdComponent::alpha("a").unwrap());

        // Invalid cases
        assert!(Id::parse("").is_err());
        assert!(Id::parse("a").is_err()); // Must start with number
        assert!(Id::parse("1A").is_err()); // No uppercase letters
        assert!(Id::parse("1-2").is_err()); // No special characters
        assert!(Id::parse("1 a").is_err()); // No spaces
    }

    #[test]
    fn test_id_display() {
        let id = Id::parse("1a2b").unwrap();
        assert_eq!(id.to_string(), "1a2b");
        assert_eq!(format!("{}", id), "1a2b");
    }

    #[test]
    fn test_id_from_str() {
        let id: Id = "1a2".parse().unwrap();
        assert_eq!(id.to_string(), "1a2");

        let result: Result<Id, _> = "invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_id_relationships() {
        let root = Id::parse("1").unwrap();
        let child = Id::parse("1a").unwrap();
        let grandchild = Id::parse("1a2").unwrap();
        let sibling = Id::parse("2").unwrap();
        let child_sibling = Id::parse("1b").unwrap();

        // Parent relationships
        assert_eq!(child.parent().unwrap(), Some(root.clone()));
        assert_eq!(grandchild.parent().unwrap(), Some(child.clone()));
        assert_eq!(root.parent().unwrap(), None);

        // Ancestor/descendant relationships
        assert!(root.is_ancestor_of(&child));
        assert!(root.is_ancestor_of(&grandchild));
        assert!(child.is_ancestor_of(&grandchild));
        assert!(!child.is_ancestor_of(&root));
        assert!(!root.is_ancestor_of(&sibling));

        assert!(child.is_descendant_of(&root));
        assert!(grandchild.is_descendant_of(&root));
        assert!(grandchild.is_descendant_of(&child));
        assert!(!root.is_descendant_of(&child));

        // Sibling relationships
        assert!(root.is_sibling_of(&sibling));
        assert!(child.is_sibling_of(&child_sibling));
        assert!(!root.is_sibling_of(&child));
        assert!(!child.is_sibling_of(&grandchild));
    }

    #[test]
    fn test_ancestors() {
        let id = Id::parse("1a2b").unwrap();
        let ancestors = id.ancestors();

        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0].to_string(), "1");
        assert_eq!(ancestors[1].to_string(), "1a");
        assert_eq!(ancestors[2].to_string(), "1a2");

        let root = Id::parse("1").unwrap();
        assert_eq!(root.ancestors().len(), 0);
    }

    #[test]
    fn test_id_generation() {
        let id = Id::parse("1a2").unwrap();

        // Next sibling
        let sibling = id.next_sibling().unwrap();
        assert_eq!(sibling.to_string(), "1a3");

        // First child follows alternating pattern
        let child = id.first_child();
        assert_eq!(child.to_string(), "1a2a"); // After number comes alpha

        // Child of child should be numeric
        let grandchild = child.first_child();
        assert_eq!(grandchild.to_string(), "1a2a1"); // After alpha comes number

        // Test root ID child
        let root = Id::parse("1").unwrap();
        let root_child = root.first_child();
        assert_eq!(root_child.to_string(), "1a"); // After number comes alpha
    }

    #[test]
    fn test_id_from_number() {
        let id = Id::from_number(42);
        assert_eq!(id.to_string(), "42");
        assert!(id.is_root());
        assert_eq!(id.depth(), 1);
    }

    #[test]
    fn test_id_manager_with_default_config() {
        let config = IdConfig::default();
        let manager = IdManager::new(config, |_| false);

        // Test fuzzy matching (default)
        assert_eq!(
            manager
                .extract_from_filename("1a2_note.md")
                .map(|id| id.to_string()),
            Some("1a2".to_string())
        );
        assert_eq!(
            manager
                .extract_from_filename("1a2-note.md")
                .map(|id| id.to_string()),
            Some("1a2".to_string())
        );
        assert_eq!(
            manager
                .extract_from_filename("1a2.md")
                .map(|id| id.to_string()),
            Some("1a2".to_string())
        );
    }

    #[test]
    fn test_id_manager_strict_matching() {
        let config = IdConfig {
            match_rule: "strict".to_string(),
            separator: " - ".to_string(),
            allow_unicode: false,
            max_depth: 10,
        };
        let manager = IdManager::new(config, |_| false);

        // Strict mode: exact match only
        assert_eq!(
            manager
                .extract_from_filename("1a2")
                .map(|id| id.to_string()),
            Some("1a2".to_string())
        );
        assert_eq!(
            manager.extract_from_filename("1a2.md"),
            None // strict mode doesn't match with extension
        );
        assert_eq!(manager.extract_from_filename("1a2-note.md"), None);
    }

    #[test]
    fn test_id_manager_separator_matching() {
        let config = IdConfig {
            match_rule: "separator".to_string(),
            separator: " - ".to_string(),
            allow_unicode: false,
            max_depth: 10,
        };
        let manager = IdManager::new(config, |_| false);

        assert_eq!(
            manager
                .extract_from_filename("1a2 - My Note.md")
                .map(|id| id.to_string()),
            Some("1a2".to_string())
        );
        assert_eq!(
            manager
                .extract_from_filename("1a2 - Another Note Title.md")
                .map(|id| id.to_string()),
            Some("1a2".to_string())
        );
        // Should not match without separator
        assert_eq!(manager.extract_from_filename("1a2.md"), None);
    }

    #[test]
    fn test_id_manager_generation_with_existing_ids() {
        use std::collections::HashSet;

        let mut existing_ids = HashSet::new();
        existing_ids.insert("1".to_string());
        existing_ids.insert("1a".to_string());
        existing_ids.insert("2".to_string());
        existing_ids.insert("3".to_string());

        let config = IdConfig::default();
        let manager = IdManager::new(config, |id: &str| existing_ids.contains(id));

        // Next available sibling of "1" should be "4" (since "2" and "3" exist)
        let current = Id::parse("1").unwrap();
        let next_sibling = manager.next_available_sibling(&current).unwrap();
        assert_eq!(next_sibling.to_string(), "4");

        // Next available child of "1" should be "1b" (since "1a" exists)
        let next_child = manager.next_available_child(&current);
        assert_eq!(next_child.to_string(), "1b");

        // Test with alpha components
        existing_ids.insert("1b".to_string());
        existing_ids.insert("1c".to_string());
        let next_child = manager.next_available_child(&current);
        assert_eq!(next_child.to_string(), "1d");
    }

    #[test]
    fn test_id_validation() {
        let config = IdConfig::default();
        let manager = IdManager::new(config, |_| false);

        // Valid IDs
        assert!(manager.validate_id("1").is_ok());
        assert!(manager.validate_id("1a").is_ok());
        assert!(manager.validate_id("1a2b3c").is_ok());
        assert!(manager.validate_id("42z").is_ok());

        // Invalid IDs
        assert!(manager.validate_id("").is_err());
        assert!(manager.validate_id("a").is_err());
        assert!(manager.validate_id("1A").is_err());
        assert!(manager.validate_id("1-2").is_err());
    }

    #[test]
    fn test_id_exists() {
        use std::collections::HashSet;

        let mut existing_ids = HashSet::new();
        existing_ids.insert("1".to_string());
        existing_ids.insert("1a2".to_string());

        let config = IdConfig::default();
        let manager = IdManager::new(config, |id: &str| existing_ids.contains(id));

        let id1 = Id::parse("1").unwrap();
        let id2 = Id::parse("1a2").unwrap();
        let id3 = Id::parse("2").unwrap();

        assert!(manager.id_exists(&id1));
        assert!(manager.id_exists(&id2));
        assert!(!manager.id_exists(&id3));
    }

    #[test]
    fn test_edge_cases() {
        // Test alphabetic sequence overflow
        let zz = IdComponent::alpha("zz").unwrap();
        let incremented = zz.increment().unwrap();
        assert_eq!(incremented, IdComponent::alpha("aaa").unwrap());

        let zzz = IdComponent::alpha("zzz").unwrap();
        let incremented = zzz.increment().unwrap();
        assert_eq!(incremented, IdComponent::alpha("aaaa").unwrap());

        // Test complex ID structure
        let complex = Id::parse("999z999z999").unwrap();
        assert_eq!(complex.depth(), 5);
        assert!(complex.is_descendant_of(&Id::parse("999").unwrap()));
        assert!(complex.is_descendant_of(&Id::parse("999z").unwrap()));
        assert!(complex.is_descendant_of(&Id::parse("999z999").unwrap()));
        assert!(complex.is_descendant_of(&Id::parse("999z999z").unwrap()));

        // Test sibling generation at different levels
        let id = Id::parse("999z999z999").unwrap();
        let sibling = id.next_sibling().unwrap();
        assert_eq!(sibling.to_string(), "999z999z1000");
    }

    #[test]
    fn test_component_parsing() {
        // Test numeric components
        let comp: IdComponent = "42".parse().unwrap();
        assert_eq!(comp, IdComponent::numeric(42));

        let comp: IdComponent = "0".parse().unwrap();
        assert_eq!(comp, IdComponent::numeric(0));

        // Test alpha components
        let comp: IdComponent = "abc".parse().unwrap();
        assert_eq!(comp, IdComponent::alpha("abc").unwrap());

        let comp: IdComponent = "z".parse().unwrap();
        assert_eq!(comp, IdComponent::alpha("z").unwrap());

        // Test invalid components
        let result: Result<IdComponent, _> = "".parse();
        assert!(result.is_err());

        let result: Result<IdComponent, _> = "ABC".parse();
        assert!(result.is_err());

        let result: Result<IdComponent, _> = "a1b".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_id_string_function() {
        // Test the internal parsing function directly
        let components = parse_id_string("1a2b").unwrap();
        assert_eq!(components.len(), 4);
        assert_eq!(components[0], IdComponent::numeric(1));
        assert_eq!(components[1], IdComponent::alpha("a").unwrap());
        assert_eq!(components[2], IdComponent::numeric(2));
        assert_eq!(components[3], IdComponent::alpha("b").unwrap());

        // Test single component
        let components = parse_id_string("42").unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0], IdComponent::numeric(42));

        let components = parse_id_string("z").unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0], IdComponent::alpha("z").unwrap());

        // Test error cases
        assert!(parse_id_string("").is_err());
        assert!(parse_id_string("1A").is_err());
        assert!(parse_id_string("a1").is_err()); // Must start with number
    }

    #[test]
    fn test_id_new_validation() {
        // Valid alternating pattern
        let components = vec![
            IdComponent::numeric(1),
            IdComponent::alpha("a").unwrap(),
            IdComponent::numeric(2),
        ];
        assert!(Id::new(components).is_ok());

        // Invalid: starts with alpha
        let components = vec![IdComponent::alpha("a").unwrap(), IdComponent::numeric(1)];
        assert!(Id::new(components).is_err());

        // Invalid: two numerics in a row
        let components = vec![IdComponent::numeric(1), IdComponent::numeric(2)];
        assert!(Id::new(components).is_err());

        // Invalid: two alphas in a row
        let components = vec![
            IdComponent::numeric(1),
            IdComponent::alpha("a").unwrap(),
            IdComponent::alpha("b").unwrap(),
        ];
        assert!(Id::new(components).is_err());

        // Invalid: empty components
        assert!(Id::new(vec![]).is_err());
    }

    #[test]
    fn test_overflow_handling() {
        // Test numeric overflow
        let large_comp = IdComponent::numeric(u32::MAX);
        assert!(large_comp.increment().is_err());

        // Test that we can still create IDs with large numbers
        let large_id = Id::from_number(u32::MAX);
        assert_eq!(large_id.to_string(), u32::MAX.to_string());

        // Test that sibling generation fails with overflow
        assert!(large_id.next_sibling().is_err());
    }
}
