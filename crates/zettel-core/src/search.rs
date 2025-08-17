// crates/zettel-core/src/search.rs - Enhanced search with indexing

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy};
use tantivy::collector::TopDocs;
use tantivy::query::{QueryParser, FuzzyTermQuery, BooleanQuery, Occur};
use tantivy::tokenizer::*;
use walkdir::WalkDir;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::id::{Id, IdManager, IdConfig};

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Index error: {0}")]
    IndexError(#[from] tantivy::TantivyError),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Query parsing error: {0}")]
    QueryError(String),
    
    #[error("Index not found, run 'zettel meta index' to create it")]
    IndexNotFound,
}

pub type SearchResult<T> = Result<T, SearchError>;

/// Represents a note in search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub path: PathBuf,
    pub score: f32,
    pub snippet: Option<String>,
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

/// Search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub index_content: bool,
    pub index_titles: bool,
    pub fuzzy_threshold: f64,
    pub max_results: usize,
    pub snippet_length: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            index_content: true,
            index_titles: true,
            fuzzy_threshold: 0.6,
            max_results: 50,
            snippet_length: 150,
        }
    }
}

/// Schema field identifiers for the search index
#[derive(Debug, Clone)]
struct SchemaFields {
    id: Field,
    title: Field,
    content: Field,
    path: Field,
    created: Field,
    modified: Field,
}

/// Fast search engine for zettelkasten notes
pub struct SearchEngine {
    index: Index,
    reader: IndexReader,
    schema: Schema,
    fields: SchemaFields,
    config: SearchConfig,
    vault_path: PathBuf,
}

impl SearchEngine {
    /// Create a new search engine with the given vault path and configuration
    pub fn new(vault_path: PathBuf, config: SearchConfig) -> SearchResult<Self> {
        let index_dir = vault_path.join(".zettel").join("search_index");
        
        // Create schema for our documents
        let mut schema_builder = Schema::builder();
        
        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT);
        let path_field = schema_builder.add_text_field("path", STRING | STORED);
        let created_field = schema_builder.add_date_field("created", STORED);
        let modified_field = schema_builder.add_date_field("modified", STORED);
        
        let schema = schema_builder.build();
        let fields = SchemaFields {
            id: id_field,
            title: title_field,
            content: content_field,
            path: path_field,
            created: created_field,
            modified: modified_field,
        };
        
        // Open or create index
        let index = if index_dir.exists() {
            Index::open_in_dir(&index_dir)?
        } else {
            fs::create_dir_all(&index_dir)?;
            Index::create_in_dir(&index_dir, schema.clone())?
        };
        
        // Set up tokenizer for better text processing
        index.tokenizers().register("default", TextAnalyzer::from(SimpleTokenizer)
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser));
        
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;
        
        Ok(Self {
            index,
            reader,
            schema,
            fields,
            config,
            vault_path,
        })
    }
    
    /// Rebuild the entire search index
    pub fn rebuild_index(&self, id_config: &IdConfig) -> SearchResult<usize> {
        let mut writer = self.index.writer(50_000_000)?; // 50MB heap
        
        // Clear existing index
        writer.delete_all_documents()?;
        
        let id_manager = IdManager::new(id_config.clone(), |_| false);
        let mut indexed_count = 0;
        
        // Walk through all markdown files in the vault
        for entry in WalkDir::new(&self.vault_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Only process markdown files
            if !path.extension().map_or(false, |ext| ext == "md") {
                continue;
            }
            
            // Skip hidden directories and system files
            if path.components().any(|c| {
                c.as_os_str().to_string_lossy().starts_with('.')
            }) {
                continue;
            }
            
            if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                // Try to extract ID from filename
                if let Some(id) = id_manager.extract_from_filename(filename) {
                    if let Ok(note_data) = self.extract_note_data(path, &id) {
                        let doc = doc!(
                            self.fields.id => note_data.id,
                            self.fields.title => note_data.title,
                            self.fields.content => note_data.content,
                            self.fields.path => note_data.path.to_string_lossy().to_string(),
                            self.fields.created => note_data.created,
                            self.fields.modified => note_data.modified,
                        );
                        
                        writer.add_document(doc)?;
                        indexed_count += 1;
                    }
                }
            }
        }
        
        writer.commit()?;
        Ok(indexed_count)
    }
    
    /// Extract note data from a file
    fn extract_note_data(&self, path: &Path, id: &Id) -> SearchResult<NoteData> {
        let content = fs::read_to_string(path)?;
        let metadata = fs::metadata(path)?;
        
        // Extract title from first heading or filename
        let title = self.extract_title(&content)
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string()
            });
        
        // Convert system time to chrono DateTime
        let created = metadata.created()
            .ok()
            .and_then(|t| chrono::DateTime::from_timestamp(
                t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64, 0
            ));
        
        let modified = metadata.modified()
            .ok()
            .and_then(|t| chrono::DateTime::from_timestamp(
                t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64, 0
            ));
        
        Ok(NoteData {
            id: id.to_string(),
            title,
            content,
            path: path.to_path_buf(),
            created,
            modified,
        })
    }
    
    /// Extract title from markdown content (first H1 heading)
    fn extract_title(&self, content: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(title) = trimmed.strip_prefix("# ") {
                return Some(title.trim().to_string());
            }
        }
        None
    }
    
    /// Search for notes matching the query
    pub fn search(&self, query: &str) -> SearchResult<Vec<SearchHit>> {
        let searcher = self.reader.searcher();
        
        // Create query parser for title and content fields
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.fields.title, self.fields.content],
        );
        
        // Parse the query - handle both simple text and complex queries
        let parsed_query = query_parser
            .parse_query(query)
            .map_err(|e| SearchError::QueryError(e.to_string()))?;
        
        // Execute search
        let top_docs = searcher.search(
            &parsed_query,
            &TopDocs::with_limit(self.config.max_results),
        )?;
        
        let mut hits = Vec::new();
        
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            
            let id = doc.get_first(self.fields.id)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();
            
            let title = doc.get_first(self.fields.title)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();
            
            let path_str = doc.get_first(self.fields.path)
                .and_then(|v| v.as_text())
                .unwrap_or("");
            
            let created = doc.get_first(self.fields.created)
                .and_then(|v| v.as_date())
                .and_then(|d| chrono::DateTime::from_timestamp(d.into_timestamp_secs(), 0));
            
            let modified = doc.get_first(self.fields.modified)
                .and_then(|v| v.as_date())
                .and_then(|d| chrono::DateTime::from_timestamp(d.into_timestamp_secs(), 0));
            
            // Generate snippet if we have content
            let snippet = self.generate_snippet(&searcher, doc_address, query)?;
            
            hits.push(SearchHit {
                id,
                title,
                path: PathBuf::from(path_str),
                score,
                snippet,
                created,
                modified,
            });
        }
        
        Ok(hits)
    }
    
    /// Generate a text snippet showing the search term in context
    fn generate_snippet(
        &self,
        searcher: &tantivy::Searcher,
        doc_address: tantivy::DocAddress,
        query: &str,
    ) -> SearchResult<Option<String>> {
        // For now, return a simple implementation
        // In a more sophisticated version, we'd use tantivy's snippet generation
        Ok(None)
    }
    
    /// Search by ID (exact match)
    pub fn search_by_id(&self, id: &str) -> SearchResult<Option<SearchHit>> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.fields.id]);
        
        let query = query_parser
            .parse_query(id)
            .map_err(|e| SearchError::QueryError(e.to_string()))?;
        
        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;
        
        if let Some((score, doc_address)) = top_docs.first() {
            let doc = searcher.doc(*doc_address)?;
            
            let title = doc.get_first(self.fields.title)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();
            
            let path_str = doc.get_first(self.fields.path)
                .and_then(|v| v.as_text())
                .unwrap_or("");
            
            let created = doc.get_first(self.fields.created)
                .and_then(|v| v.as_date())
                .and_then(|d| chrono::DateTime::from_timestamp(d.into_timestamp_secs(), 0));
            
            let modified = doc.get_first(self.fields.modified)
                .and_then(|v| v.as_date())
                .and_then(|d| chrono::DateTime::from_timestamp(d.into_timestamp_secs(), 0));
            
            Ok(Some(SearchHit {
                id: id.to_string(),
                title,
                path: PathBuf::from(path_str),
                score: *score,
                snippet: None,
                created,
                modified,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Get statistics about the index
    pub fn get_stats(&self) -> SearchResult<IndexStats> {
        let searcher = self.reader.searcher();
        let num_docs = searcher.num_docs() as usize;
        
        Ok(IndexStats {
            num_documents: num_docs,
            index_size_bytes: self.get_index_size()?,
        })
    }
    
    /// Get the size of the index directory in bytes
    fn get_index_size(&self) -> SearchResult<u64> {
        let index_dir = self.vault_path.join(".zettel").join("search_index");
        let mut total_size = 0;
        
        for entry in WalkDir::new(&index_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                total_size += entry.metadata()?.len();
            }
        }
        
        Ok(total_size)
    }
}

/// Internal struct for note data extraction
#[derive(Debug)]
struct NoteData {
    id: String,
    title: String,
    content: String,
    path: PathBuf,
    created: Option<chrono::DateTime<chrono::Utc>>,
    modified: Option<chrono::DateTime<chrono::Utc>>,
}

/// Index statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexStats {
    pub num_documents: usize,
    pub index_size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_vault() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().to_path_buf();
        
        // Create .zettel directory
        fs::create_dir_all(vault_path.join(".zettel")).unwrap();
        
        // Create some test notes
        fs::write(vault_path.join("1.md"), "# First Note\n\nThis is the first note about philosophy.").unwrap();
        fs::write(vault_path.join("1a.md"), "# Child Note\n\nThis is a child note about epistemology.").unwrap();
        fs::write(vault_path.join("2.md"), "# Second Note\n\nThis note discusses methodology.").unwrap();
        
        (temp_dir, vault_path)
    }

    #[test]
    fn test_search_engine_creation() {
        let (_temp_dir, vault_path) = create_test_vault();
        let config = SearchConfig::default();
        
        let engine = SearchEngine::new(vault_path, config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_index_rebuild() {
        let (_temp_dir, vault_path) = create_test_vault();
        let config = SearchConfig::default();
        let engine = SearchEngine::new(vault_path, config).unwrap();
        
        let id_config = IdConfig::default();
        let result = engine.rebuild_index(&id_config);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3); // Should index 3 test notes
    }

    #[test]
    fn test_search_functionality() {
        let (_temp_dir, vault_path) = create_test_vault();
        let config = SearchConfig::default();
        let engine = SearchEngine::new(vault_path, config).unwrap();
        
        let id_config = IdConfig::default();
        engine.rebuild_index(&id_config).unwrap();
        
        // Search for philosophy
        let results = engine.search("philosophy").unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|hit| hit.id == "1"));
        
        // Search for methodology
        let results = engine.search("methodology").unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|hit| hit.id == "2"));
    }

    #[test]
    fn test_search_by_id() {
        let (_temp_dir, vault_path) = create_test_vault();
        let config = SearchConfig::default();
        let engine = SearchEngine::new(vault_path, config).unwrap();
        
        let id_config = IdConfig::default();
        engine.rebuild_index(&id_config).unwrap();
        
        let result = engine.search_by_id("1a").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().title, "Child Note");
    }
}
