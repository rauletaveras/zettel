//! # Zettel CLI
//!
//! Unix-style command-line interface for Zettelkasten management.
//!
//! ## Design Philosophy
//!
//! - **Composable**: Each subcommand does one thing well
//! - **Scriptable**: Machine-readable output formats (JSON, CSV)
//! - **Fast**: Efficient operations with caching and indexing
//! - **Reliable**: Atomic operations with rollback support
//!
//! ## Command Structure
//!
//! ```
//! zettel <subcommand> [options] [arguments]
//! ```
//!
//! Each subcommand follows Unix conventions:
//! - Returns 0 on success, non-zero on error
//! - Outputs to stdout, errors to stderr
//! - Respects environment variables and config files
//! - Provides both human and machine-readable output

use anyhow::{Context, anyhow};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use zettel_core::{self as zettel, Config, NoteFilter, Result, SearchQuery, Vault, VaultBuilder};

/// Zettelkasten management CLI
#[derive(Parser)]
#[command(
    name = "zettel",
    about = "A composable CLI for Luhmann-style Zettelkasten management",
    long_about = "
Zettel provides Unix-style commands for managing a Zettelkasten note system.
Each command is designed to do one thing well and compose with other tools.

Examples:
  zettel init ~/notes                    # Initialize new vault
  zettel note create 1 'First Note'     # Create a note
  zettel id next-sibling 1a              # Generate next sibling ID
  zettel search 'philosophy' --json     # Search with JSON output
  zettel list --orphans | wc -l         # Count orphaned notes

Environment Variables:
  ZETTEL_VAULT      Default vault path
  ZETTEL_EDITOR     Editor for opening notes
  ZETTEL_CONFIG     Configuration file path
"
)]
#[command(version, author)]
struct Cli {
    /// Vault directory (overrides ZETTEL_VAULT)
    #[arg(short, long, global = true)]
    vault: Option<PathBuf>,

    /// Output format
    #[arg(short = 'f', long, global = true, default_value = "human")]
    format: OutputFormat,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Dry run (show what would be done)
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new vault
    Init {
        /// Path to create vault (defaults to current directory)
        path: Option<PathBuf>,

        /// Configuration template to use
        #[arg(long)]
        template: Option<String>,

        /// Force initialization even if directory exists
        #[arg(long)]
        force: bool,
    },

    /// Vault operations
    #[command(subcommand)]
    Vault(VaultCommands),

    /// ID operations
    #[command(subcommand)]
    Id(IdCommands),

    /// Note operations  
    #[command(subcommand)]
    Note(NoteCommands),

    /// Search operations
    #[command(subcommand)]
    Search(SearchCommands),

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

    /// List notes with optional filtering
    List {
        /// Filter pattern (glob or regex)
        pattern: Option<String>,

        /// Show only orphaned notes
        #[arg(long)]
        orphans: bool,

        /// Show only notes with children
        #[arg(long)]
        parents: bool,

        /// Filter by parent ID
        #[arg(long)]
        parent: Option<String>,

        /// Filter by tags
        #[arg(long)]
        tags: Option<Vec<String>>,

        /// Only count results
        #[arg(long)]
        count: bool,

        /// Show full paths
        #[arg(long)]
        full_paths: bool,
    },

    /// Show note hierarchy as tree
    Tree {
        /// Root note ID (default: show all roots)
        root: Option<String>,

        /// Maximum depth to show
        #[arg(short, long)]
        depth: Option<usize>,

        /// Show note titles
        #[arg(short, long)]
        titles: bool,
    },

    /// Generate relationship graph
    Graph {
        /// Output format
        #[arg(long, default_value = "dot")]
        format: GraphFormat,

        /// Root node for subgraph
        #[arg(long)]
        root: Option<String>,

        /// Maximum depth from root
        #[arg(long)]
        depth: Option<usize>,

        /// Include orphaned notes
        #[arg(long)]
        include_orphans: bool,
    },

    /// Validate vault integrity
    Validate {
        /// Fix issues automatically where possible
        #[arg(long)]
        fix: bool,

        /// Only show errors (not warnings)
        #[arg(long)]
        errors_only: bool,
    },

    /// Import from other systems
    Import {
        /// Source system type
        #[arg(value_enum)]
        source: ImportSource,

        /// Source path
        path: PathBuf,

        /// Mapping configuration file
        #[arg(long)]
        mapping: Option<PathBuf>,

        /// Preserve original file structure
        #[arg(long)]
        preserve_structure: bool,
    },

    /// Export to other formats
    Export {
        /// Target format
        #[arg(value_enum)]
        target: ExportTarget,

        /// Output path
        output: PathBuf,

        /// Include assets (images, etc.)
        #[arg(long)]
        include_assets: bool,

        /// Export template
        #[arg(long)]
        template: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum VaultCommands {
    /// Show vault status and statistics
    Status,

    /// Rebuild search index
    Reindex {
        /// Force full rebuild (not incremental)
        #[arg(long)]
        full: bool,
    },

    /// Create backup of vault
    Backup {
        /// Backup destination
        destination: Option<PathBuf>,

        /// Compress backup
        #[arg(long)]
        compress: bool,
    },

    /// Restore from backup
    Restore {
        /// Backup file to restore
        backup: PathBuf,

        /// Restore to different location
        #[arg(long)]
        destination: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum IdCommands {
    /// Generate next sibling ID
    NextSibling {
        /// Current ID
        id: String,
    },

    /// Generate next child ID
    NextChild {
        /// Parent ID
        id: String,
    },

    /// Validate ID format
    Validate {
        /// ID to validate
        id: String,
    },

    /// Parse ID from filename
    Parse {
        /// Filename to parse
        filename: String,
    },

    /// Show ID hierarchy
    Hierarchy {
        /// ID to show hierarchy for
        id: String,

        /// Show ancestors
        #[arg(long)]
        ancestors: bool,

        /// Show descendants  
        #[arg(long)]
        descendants: bool,
    },
}

#[derive(Subcommand)]
enum NoteCommands {
    /// Create a new note
    Create {
        /// Note ID
        id: String,

        /// Note title
        title: Option<String>,

        /// Template to use
        #[arg(short, long)]
        template: Option<String>,

        /// Parent note for linking
        #[arg(short, long)]
        parent: Option<String>,

        /// Open in editor after creation
        #[arg(long)]
        open: bool,
    },

    /// Update an existing note
    Update {
        /// Note ID
        id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// Add tags
        #[arg(long)]
        add_tags: Option<Vec<String>>,

        /// Remove tags
        #[arg(long)]
        remove_tags: Option<Vec<String>>,
    },

    /// Delete a note
    Delete {
        /// Note ID
        id: String,

        /// Force deletion without confirmation
        #[arg(long)]
        force: bool,

        /// Delete children recursively
        #[arg(long)]
        recursive: bool,
    },

    /// Show note information
    Show {
        /// Note ID
        id: String,

        /// Show content
        #[arg(short, long)]
        content: bool,

        /// Show metadata only
        #[arg(short, long)]
        metadata: bool,
    },

    /// Open note in editor
    Open {
        /// Note ID or search query
        target: String,

        /// Editor to use (overrides ZETTEL_EDITOR)
        #[arg(long)]
        editor: Option<String>,
    },

    /// Create link between notes
    Link {
        /// Source note ID
        from: String,

        /// Target note ID
        to: String,

        /// Link type
        #[arg(long, default_value = "bidirectional")]
        link_type: LinkType,
    },

    /// Remove link between notes
    Unlink {
        /// Source note ID
        from: String,

        /// Target note ID
        to: String,
    },
}

#[derive(Subcommand)]
enum SearchCommands {
    /// Search notes by content or metadata
    Query {
        /// Search query
        query: String,

        /// Search in titles only
        #[arg(long)]
        titles_only: bool,

        /// Search in content only
        #[arg(long)]
        content_only: bool,

        /// Case sensitive search
        #[arg(long)]
        case_sensitive: bool,

        /// Use regex
        #[arg(long)]
        regex: bool,

        /// Maximum results
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Find similar notes
    Similar {
        /// Note ID to find similar notes for
        id: String,

        /// Similarity threshold (0.0-1.0)
        #[arg(long, default_value = "0.7")]
        threshold: f64,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Interactive fuzzy search
    Interactive {
        /// Initial query
        query: Option<String>,

        /// Search scope
        #[arg(long, default_value = "all")]
        scope: SearchScope,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show {
        /// Show only specified key
        key: Option<String>,

        /// Show default values
        #[arg(long)]
        defaults: bool,
    },

    /// Set configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,

        /// Set globally (not vault-specific)
        #[arg(long)]
        global: bool,
    },

    /// Unset configuration value
    Unset {
        /// Configuration key
        key: String,

        /// Unset globally
        #[arg(long)]
        global: bool,
    },

    /// Validate configuration
    Validate,

    /// Reset configuration to defaults
    Reset {
        /// Reset global configuration
        #[arg(long)]
        global: bool,

        /// Force reset without confirmation
        #[arg(long)]
        force: bool,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum OutputFormat {
    Human,
    Json,
    Csv,
    Yaml,
    Xml,
}

#[derive(ValueEnum, Clone, Debug)]
enum GraphFormat {
    Dot,
    Json,
    Svg,
    Png,
    Mermaid,
}

#[derive(ValueEnum, Clone, Debug)]
enum ImportSource {
    Obsidian,
    Zettlr,
    Roam,
    Logseq,
    Markdown,
    Filesystem,
}

#[derive(ValueEnum, Clone, Debug)]
enum ExportTarget {
    Obsidian,
    Hugo,
    Jekyll,
    Gatsby,
    Json,
    Zip,
}

#[derive(ValueEnum, Clone, Debug)]
enum LinkType {
    Unidirectional,
    Bidirectional,
    Parent,
    Child,
}

#[derive(ValueEnum, Clone, Debug)]
enum SearchScope {
    All,
    Titles,
    Content,
    Tags,
    Links,
}

struct Context {
    vault: Vault,
    config: Config,
    format: OutputFormat,
    colored: bool,
    verbose: bool,
    dry_run: bool,
}

impl Context {
    fn new(cli: &Cli) -> Result<Self> {
        let vault_path = cli
            .vault
            .clone()
            .or_else(|| env::var("ZETTEL_VAULT").ok().map(PathBuf::from))
            .unwrap_or_else(|| env::current_dir().unwrap());

        let config = Config::load_from_path(&vault_path).unwrap_or_default();

        let vault = VaultBuilder::new(&vault_path)
            .with_config(config.clone())
            .build()
            .with_context(|| format!("Failed to open vault at {}", vault_path.display()))?;

        Ok(Context {
            vault,
            config,
            format: cli.format.clone(),
            colored: !cli.no_color && atty::is(atty::Stream::Stdout),
            verbose: cli.verbose,
            dry_run: cli.dry_run,
        })
    }

    fn output<T: serde::Serialize>(&self, data: &T) -> Result<()> {
        match self.format {
            OutputFormat::Human => {
                // Custom human-readable formatting per command
                println!("{:#?}", data);
            }
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(data)?);
            }
            OutputFormat::Csv => {
                // Would implement CSV writer
                todo!("CSV output not yet implemented");
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(data)?);
            }
            OutputFormat::Xml => {
                todo!("XML output not yet implemented");
            }
        }
        Ok(())
    }

    fn verbose(&self, msg: &str) {
        if self.verbose {
            eprintln!("zettel: {}", msg);
        }
    }

    fn error(&self, msg: &str) {
        if self.colored {
            eprintln!("\x1b[31merror:\x1b[0m {}", msg);
        } else {
            eprintln!("error: {}", msg);
        }
    }

    fn warn(&self, msg: &str) {
        if self.colored {
            eprintln!("\x1b[33mwarning:\x1b[0m {}", msg);
        } else {
            eprintln!("warning: {}", msg);
        }
    }

    fn success(&self, msg: &str) {
        if self.colored {
            eprintln!("\x1b[32m{}\x1b[0m", msg);
        } else {
            eprintln!("{}", msg);
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle commands that don't need a vault context
    match &cli.command {
        Commands::Init {
            path,
            template,
            force,
        } => {
            return handle_init(path.clone(), template.clone(), *force);
        }
        _ => {}
    }

    let ctx = Context::new(&cli)?;

    let result = match cli.command {
        Commands::Vault(cmd) => handle_vault_command(&ctx, cmd),
        Commands::Id(cmd) => handle_id_command(&ctx, cmd),
        Commands::Note(cmd) => handle_note_command(&ctx, cmd),
        Commands::Search(cmd) => handle_search_command(&ctx, cmd),
        Commands::Config(cmd) => handle_config_command(&ctx, cmd),
        Commands::List {
            pattern,
            orphans,
            parents,
            parent,
            tags,
            count,
            full_paths,
        } => handle_list_command(
            &ctx, pattern, orphans, parents, parent, tags, count, full_paths,
        ),
        Commands::Tree {
            root,
            depth,
            titles,
        } => handle_tree_command(&ctx, root, depth, titles),
        Commands::Graph {
            format,
            root,
            depth,
            include_orphans,
        } => handle_graph_command(&ctx, format, root, depth, include_orphans),
        Commands::Validate { fix, errors_only } => handle_validate_command(&ctx, fix, errors_only),
        Commands::Import {
            source,
            path,
            mapping,
            preserve_structure,
        } => handle_import_command(&ctx, source, path, mapping.as_ref(), preserve_structure),
        Commands::Export {
            target,
            output,
            include_assets,
            template,
        } => handle_export_command(&ctx, target, output, include_assets, template.as_ref()),
        Commands::Init { .. } => unreachable!(), // Handled above
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            ctx.error(&format!("{}", e));
            std::process::exit(1);
        }
    }
}

fn handle_init(path: Option<PathBuf>, template: Option<String>, force: bool) -> anyhow::Result<()> {
    let vault_path = path.unwrap_or_else(|| env::current_dir().unwrap());

    if vault_path.exists() && !force {
        return Err(anyhow!(
            "Directory already exists. Use --force to initialize anyway."
        ));
    }

    let config = if let Some(template_name) = template {
        Config::from_template(&template_name)?
    } else {
        Config::default()
    };

    let vault = VaultBuilder::new(&vault_path)
        .with_config(config)
        .initialize_if_missing(true)
        .build()?;

    println!("Initialized zettelkasten vault at {}", vault_path.display());
    Ok(())
}

fn handle_vault_command(ctx: &Context, cmd: VaultCommands) -> Result<()> {
    match cmd {
        VaultCommands::Status => {
            let stats = ctx.vault.stats()?;
            ctx.output(&stats)?;
        }
        VaultCommands::Reindex { full } => {
            ctx.verbose("Rebuilding search index...");
            if full {
                ctx.vault.reindex_full()?;
            } else {
                ctx.vault.reindex()?;
            }
            ctx.success("Search index rebuilt");
        }
        VaultCommands::Backup {
            destination,
            compress,
        } => {
            let backup_path = ctx.vault.create_backup(destination.as_ref(), compress)?;
            println!("Backup created: {}", backup_path.display());
        }
        VaultCommands::Restore {
            backup,
            destination,
        } => {
            ctx.vault.restore_backup(&backup, destination.as_ref())?;
            ctx.success("Vault restored from backup");
        }
    }
    Ok(())
}

fn handle_id_command(ctx: &Context, cmd: IdCommands) -> Result<()> {
    let id_manager = ctx.vault.id_manager();

    match cmd {
        IdCommands::NextSibling { id } => {
            let next_id = id_manager.next_sibling(&id)?;
            println!("{}", next_id);
        }
        IdCommands::NextChild { id } => {
            let next_id = id_manager.next_child(&id)?;
            println!("{}", next_id);
        }
        IdCommands::Validate { id } => {
            if id_manager.validate(&id)? {
                println!("valid");
            } else {
                println!("invalid");
                std::process::exit(1);
            }
        }
        IdCommands::Parse { filename } => {
            if let Some(id) = id_manager.parse_filename(&filename)? {
                println!("{}", id);
            } else {
                std::process::exit(1);
            }
        }
        IdCommands::Hierarchy {
            id,
            ancestors,
            descendants,
        } => {
            let hierarchy = id_manager.get_hierarchy(&id, ancestors, descendants)?;
            ctx.output(&hierarchy)?;
        }
    }
    Ok(())
}

fn handle_note_command(ctx: &Context, cmd: NoteCommands) -> Result<()> {
    match cmd {
        NoteCommands::Create {
            id,
            title,
            template,
            parent,
            open,
        } => {
            if ctx.dry_run {
                println!("Would create note: {} with title: {:?}", id, title);
                return Ok(());
            }

            let note = ctx.vault.create_note(&id, title.as_deref())?;

            if let Some(template_name) = template {
                ctx.vault.apply_template(&note, &template_name)?;
            }

            if let Some(parent_id) = parent {
                ctx.vault.create_link(&parent_id, &id, LinkType::Parent)?;
            }

            ctx.success(&format!("Created note: {}", id));

            if open {
                let editor = env::var("ZETTEL_EDITOR").unwrap_or_else(|_| "vim".to_string());
                std::process::Command::new(editor)
                    .arg(note.path())
                    .status()?;
            }
        }
        NoteCommands::Update {
            id,
            title,
            add_tags,
            remove_tags,
        } => {
            let mut note = ctx.vault.get_note(&id)?;

            if let Some(new_title) = title {
                note.set_title(Some(&new_title));
            }

            if let Some(tags) = add_tags {
                for tag in tags {
                    note.add_tag(&tag);
                }
            }

            if let Some(tags) = remove_tags {
                for tag in tags {
                    note.remove_tag(&tag);
                }
            }

            ctx.vault.update_note(&note)?;
            ctx.success(&format!("Updated note: {}", id));
        }
        NoteCommands::Delete {
            id,
            force,
            recursive,
        } => {
            if !force {
                print!("Delete note '{}'? [y/N] ", id);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if !input.trim().to_lowercase().starts_with('y') {
                    return Ok(());
                }
            }

            ctx.vault.delete_note(&id, recursive)?;
            ctx.success(&format!("Deleted note: {}", id));
        }
        NoteCommands::Show {
            id,
            content,
            metadata,
        } => {
            let note = ctx.vault.get_note(&id)?;

            if metadata || (!content && !metadata) {
                let meta = note.metadata();
                ctx.output(&meta)?;
            }

            if content {
                println!("{}", note.content());
            }
        }
        NoteCommands::Open { target, editor } => {
            let note = if target.len() <= 10 && ctx.vault.note_exists(&target) {
                // Treat as ID
                ctx.vault.get_note(&target)?
            } else {
                // Search by title
                let results = ctx.vault.search(&SearchQuery::new(&target))?;
                if results.is_empty() {
                    return Err(zettel::ZettelError::NoteNotFound(target));
                }
                ctx.vault.get_note(&results[0].id)?
            };

            let editor_cmd = editor
                .or_else(|| env::var("ZETTEL_EDITOR").ok())
                .unwrap_or_else(|| "vim".to_string());

            std::process::Command::new(editor_cmd)
                .arg(note.path())
                .status()?;
        }
        NoteCommands::Link {
            from,
            to,
            link_type,
        } => {
            ctx.vault.create_link(&from, &to, link_type)?;
            ctx.success(&format!("Created link: {} -> {}", from, to));
        }
        NoteCommands::Unlink { from, to } => {
            ctx.vault.remove_link(&from, &to)?;
            ctx.success(&format!("Removed link: {} -> {}", from, to));
        }
    }
    Ok(())
}

fn handle_search_command(ctx: &Context, cmd: SearchCommands) -> Result<()> {
    match cmd {
        SearchCommands::Query {
            query,
            titles_only,
            content_only,
            case_sensitive,
            regex,
            limit,
        } => {
            let mut search_query = SearchQuery::new(&query);

            if titles_only {
                search_query = search_query.titles_only();
            }
            if content_only {
                search_query = search_query.content_only();
            }
            if case_sensitive {
                search_query = search_query.case_sensitive();
            }
            if regex {
                search_query = search_query.regex();
            }
            if let Some(limit) = limit {
                search_query = search_query.limit(limit);
            }

            let results = ctx.vault.search(&search_query)?;
            ctx.output(&results)?;
        }
        SearchCommands::Similar {
            id,
            threshold,
            limit,
        } => {
            let results = ctx.vault.find_similar(&id, threshold, limit)?;
            ctx.output(&results)?;
        }
        SearchCommands::Interactive { query, scope } => {
            // This would integrate with external tools like fzf
            todo!("Interactive search not yet implemented");
        }
    }
    Ok(())
}

fn handle_config_command(ctx: &Context, cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::Show { key, defaults } => {
            if defaults {
                let default_config = Config::default();
                ctx.output(&default_config)?;
            } else if let Some(key) = key {
                let value = ctx.config.get(&key)?;
                println!("{}", value);
            } else {
                ctx.output(&ctx.config)?;
            }
        }
        ConfigCommands::Set { key, value, global } => {
            if global {
                Config::set_global(&key, &value)?;
            } else {
                ctx.vault.set_config(&key, &value)?;
            }
            ctx.success(&format!("Set {}: {}", key, value));
        }
        ConfigCommands::Unset { key, global } => {
            if global {
                Config::unset_global(&key)?;
            } else {
                ctx.vault.unset_config(&key)?;
            }
            ctx.success(&format!("Unset {}", key));
        }
        ConfigCommands::Validate => {
            let issues = ctx.config.validate()?;
            if issues.is_empty() {
                ctx.success("Configuration is valid");
            } else {
                ctx.output(&issues)?;
                std::process::exit(1);
            }
        }
        ConfigCommands::Reset { global, force } => {
            if !force {
                print!("Reset configuration to defaults? [y/N] ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if !input.trim().to_lowercase().starts_with('y') {
                    return Ok(());
                }
            }

            if global {
                Config::reset_global()?;
            } else {
                ctx.vault.reset_config()?;
            }
            ctx.success("Configuration reset to defaults");
        }
    }
    Ok(())
}

fn handle_list_command(
    ctx: &Context,
    pattern: Option<String>,
    orphans: bool,
    parents: bool,
    parent: Option<String>,
    tags: Option<Vec<String>>,
    count: bool,
    full_paths: bool,
) -> Result<()> {
    let mut filter = NoteFilter::default();

    if orphans {
        filter.orphaned = Some(true);
    }
    if parents {
        filter.has_children = Some(true);
    }
    if let Some(parent_id) = parent {
        filter.parent = Some(parent_id);
    }
    if let Some(tag_list) = tags {
        filter.tags = Some(tag_list);
    }

    let notes = ctx.vault.list_notes(Some(&filter))?;

    let filtered_notes: Vec<_> = if let Some(pattern) = pattern {
        let regex = regex::Regex::new(&pattern)?;
        notes
            .into_iter()
            .filter(|note| {
                regex.is_match(&note.id) || note.title.as_ref().map_or(false, |t| regex.is_match(t))
            })
            .collect()
    } else {
        notes
    };

    if count {
        println!("{}", filtered_notes.len());
    } else {
        ctx.output(&filtered_notes)?;
    }

    Ok(())
}

fn handle_tree_command(
    ctx: &Context,
    root: Option<String>,
    depth: Option<usize>,
    titles: bool,
) -> Result<()> {
    let tree = ctx.vault.generate_tree(root.as_deref(), depth, titles)?;
    ctx.output(&tree)?;
    Ok(())
}

fn handle_graph_command(
    ctx: &Context,
    format: GraphFormat,
    root: Option<String>,
    depth: Option<usize>,
    include_orphans: bool,
) -> Result<()> {
    let graph = ctx
        .vault
        .generate_graph(root.as_deref(), depth, include_orphans)?;

    match format {
        GraphFormat::Dot => {
            println!("{}", graph.to_dot());
        }
        GraphFormat::Json => {
            ctx.output(&graph)?;
        }
        GraphFormat::Svg => {
            let svg = graph.to_svg()?;
            println!("{}", svg);
        }
        GraphFormat::Png => {
            let png_data = graph.to_png()?;
            io::stdout().write_all(&png_data)?;
        }
        GraphFormat::Mermaid => {
            println!("{}", graph.to_mermaid());
        }
    }

    Ok(())
}

fn handle_validate_command(ctx: &Context, fix: bool, errors_only: bool) -> Result<()> {
    let issues = ctx.vault.validate()?;

    let filtered_issues: Vec<_> = if errors_only {
        issues
            .into_iter()
            .filter(|issue| matches!(issue.severity, zettel::IssueSeverity::Error))
            .collect()
    } else {
        issues
    };

    if filtered_issues.is_empty() {
        ctx.success("No issues found");
        return Ok(());
    }

    ctx.output(&filtered_issues)?;

    if fix {
        let fixed = ctx.vault.fix_issues(&filtered_issues)?;
        ctx.success(&format!("Fixed {} issues", fixed));
    }

    // Exit with error code if issues found
    if filtered_issues
        .iter()
        .any(|i| matches!(i.severity, zettel::IssueSeverity::Error))
    {
        std::process::exit(1);
    }

    Ok(())
}

fn handle_import_command(
    ctx: &Context,
    source: ImportSource,
    path: &PathBuf,
    mapping: Option<&PathBuf>,
    preserve_structure: bool,
) -> Result<()> {
    ctx.verbose(&format!("Importing from {:?}: {}", source, path.display()));

    let imported = ctx
        .vault
        .import(source, path, mapping, preserve_structure)?;
    ctx.success(&format!("Imported {} notes", imported));

    Ok(())
}

fn handle_export_command(
    ctx: &Context,
    target: ExportTarget,
    output: &PathBuf,
    include_assets: bool,
    template: Option<&PathBuf>,
) -> Result<()> {
    ctx.verbose(&format!("Exporting to {:?}: {}", target, output.display()));

    let exported = ctx.vault.export(target, output, include_assets, template)?;
    ctx.success(&format!("Exported {} notes", exported));

    Ok(())
}
