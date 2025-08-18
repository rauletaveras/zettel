# Zettel roadmap for parity with obsidian-luhman-plugin
## ✅ What We Have
### Core ID System:

- Complete Luhmann ID parsing and generation (zettel-core/src/id.rs)
- Next sibling generation (zettel id next-sibling)
- Next child generation (zettel id next-child)
- ID validation (zettel id validate)
- ID extraction from filenames (zettel id parse)
- Configurable matching rules (strict/separator/fuzzy)

### Basic Infrastructure:

- CLI framework with proper command structure
- Configuration system (though minimal)
- File operations service
- Editor integration service
- Vault initialization (zettel init)

### Simple Operations:

- Note listing (zettel list)
- Basic search (zettel search)
- Note creation (zettel note create)
- Note display (zettel note show)
- File opening (zettel note open)

## ❌ What We're Missing for Feature Parity
### Template System:

- Custom template file support
- Template validation (checking for {{title}} and {{link}} placeholders)
- Template content generation with placeholder substitution
- Built-in vs custom template logic

### Bidirectional Linking:

- Automatic parent→child link insertion when creating children
- Automatic child→parent backlink insertion
- Link format configuration (with/without aliases)
- Text selection replacement with links

### Advanced Note Operations:

- Sibling/child creation with automatic linking
- Frontmatter alias generation
- Link insertion at cursor position
- Text selection processing as titles

### Hierarchy Management:

- Note renaming/moving operations
- Child note cascading moves
- Outdenting (moving notes up hierarchy levels)
- Conflict resolution when moving notes

### Navigation & Discovery:

- Fuzzy search with title suggestions
- Parent note navigation
- Zettel file filtering (excluding system files)
- Title extraction from note content

### Performance Optimizations:

- Caching of zettel file lists
- Metadata cache usage for title extraction
- Cache invalidation strategies

### Configuration Completeness:

- Filename formatting options (ID-only vs ID+title)
- Link behavior settings (insert in parent/child)
- Separator configuration
- Alias generation settings
