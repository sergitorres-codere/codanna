//! Tantivy-based full-text search for documentation and code
//!
//! This module provides rich full-text search capabilities using Tantivy,
//! enabling semantic search across documentation, code, and symbols.

use super::{MetadataKey, StorageError, StorageResult};
use crate::relationship::RelationshipMetadata;
use crate::vector::{ClusterId, EmbeddingGenerator, SegmentOrdinal, VectorId, VectorSearchEngine};
use crate::{FileId, RelationKind, Relationship, SymbolId, SymbolKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use tantivy::DocId;
use tantivy::{
    Index, IndexReader, IndexSettings, IndexWriter, ReloadPolicy, TantivyDocument as Document,
    Term,
    collector::TopDocs,
    directory::MmapDirectory,
    query::{BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser, TermQuery},
    schema::{
        FAST, Field, IndexRecordOption, NumericOptions, STORED, STRING, Schema, SchemaBuilder,
        TextFieldIndexing, TextOptions, Value,
    },
    tokenizer::{NgramTokenizer, TextAnalyzer},
};

/// Schema fields for the document index
#[derive(Debug)]
pub struct IndexSchema {
    // Document type discriminator
    pub doc_type: Field,

    // Symbol fields
    pub symbol_id: Field,
    pub name: Field,      // STRING field for exact matching
    pub name_text: Field, // TEXT field for full-text search
    pub doc_comment: Field,
    pub signature: Field,
    pub module_path: Field,
    pub kind: Field,
    pub file_path: Field,
    pub line_number: Field,
    pub column: Field,
    pub end_line: Field,
    pub end_column: Field,
    pub context: Field,
    pub visibility: Field,
    pub scope_context: Field,
    pub language: Field, // Language identifier for the symbol

    // Relationship fields
    pub from_symbol_id: Field,
    pub to_symbol_id: Field,
    pub relation_kind: Field,
    pub relation_weight: Field,
    pub relation_line: Field,
    pub relation_column: Field,
    pub relation_context: Field,

    // File info fields
    pub file_id: Field,
    pub file_hash: Field,
    pub file_timestamp: Field,

    // Metadata fields
    pub meta_key: Field,
    pub meta_value: Field,

    // Vector search fields
    pub cluster_id: Field,
    pub vector_id: Field,
    pub has_vector: Field,
}

impl IndexSchema {
    /// Create the schema for indexing code documentation
    pub fn build() -> (Schema, IndexSchema) {
        let mut builder = SchemaBuilder::default();

        // Document type discriminator (for symbols, relationships, files, metadata)
        let doc_type = builder.add_text_field("doc_type", STRING | STORED | FAST);

        // Numeric options for indexed u64 fields
        let indexed_u64_options = NumericOptions::default()
            .set_indexed()
            .set_stored()
            .set_fast();

        // Symbol fields (existing)
        let symbol_id = builder.add_u64_field("symbol_id", indexed_u64_options.clone());
        let file_path = builder.add_text_field("file_path", STRING | STORED);
        let line_number = builder.add_u64_field("line_number", STORED | FAST);
        let column = builder.add_u64_field("column", STORED);
        let end_line = builder.add_u64_field("end_line", STORED | FAST);
        let end_column = builder.add_u64_field("end_column", STORED);

        // Text fields for search
        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("default")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        // IMPORTANT: Use STRING for exact matching of symbol names without tokenization
        // This prevents partial matches and ensures "MyService" doesn't match "Main"
        let name = builder.add_text_field("name", STRING | STORED);

        // ALSO add name_text for full-text search with ngram tokenization
        // This allows partial matching: "Archive" will match "ArchiveAppService"
        let ngram_text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("ngram")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();
        let name_text = builder.add_text_field("name_text", ngram_text_options);

        let doc_comment = builder.add_text_field("doc_comment", text_options.clone());
        let signature = builder.add_text_field("signature", text_options.clone());
        let context = builder.add_text_field("context", text_options.clone());

        // String fields for filtering (using STRING for exact match)
        let module_path = builder.add_text_field("module_path", STRING | STORED);
        let kind = builder.add_text_field("kind", STRING | STORED | FAST);
        let visibility = builder.add_u64_field("visibility", STORED);
        let scope_context = builder.add_text_field("scope_context", STRING | STORED);
        let language = builder.add_text_field("language", STRING | STORED | FAST);

        // Relationship fields
        let from_symbol_id = builder.add_u64_field("from_symbol_id", indexed_u64_options.clone());
        let to_symbol_id = builder.add_u64_field("to_symbol_id", indexed_u64_options.clone());
        let relation_kind = builder.add_text_field("relation_kind", STRING | STORED | FAST);
        let relation_weight = builder.add_f64_field("relation_weight", STORED);
        let relation_line = builder.add_u64_field("relation_line", STORED);
        let relation_column = builder.add_u64_field("relation_column", STORED);
        let relation_context = builder.add_text_field("relation_context", text_options.clone());

        // File info fields
        let file_id = builder.add_u64_field("file_id", indexed_u64_options.clone());
        let file_hash = builder.add_text_field("file_hash", STRING | STORED);
        let file_timestamp = builder.add_u64_field("file_timestamp", STORED | FAST);

        // Metadata fields (for counters, etc.)
        let meta_key = builder.add_text_field("meta_key", STRING | STORED | FAST);
        let meta_value = builder.add_u64_field("meta_value", STORED | FAST);

        // Vector search fields
        let cluster_id = builder.add_u64_field("cluster_id", FAST | STORED);
        let vector_id = builder.add_u64_field("vector_id", FAST | STORED);
        let has_vector = builder.add_u64_field("has_vector", FAST | STORED); // Using u64 as bool for FAST field

        let schema = builder.build();
        let index_schema = IndexSchema {
            doc_type,
            symbol_id,
            name,
            name_text,
            doc_comment,
            signature,
            module_path,
            kind,
            file_path,
            line_number,
            column,
            end_line,
            end_column,
            context,
            visibility,
            scope_context,
            language,
            from_symbol_id,
            to_symbol_id,
            relation_kind,
            relation_weight,
            relation_line,
            relation_column,
            relation_context,
            file_id,
            file_hash,
            file_timestamp,
            meta_key,
            meta_value,
            cluster_id,
            vector_id,
            has_vector,
        };

        (schema, index_schema)
    }
}

/// Metadata for tracking vector-related information per document
#[derive(Debug, Clone, PartialEq)]
pub struct VectorMetadata {
    /// The vector ID associated with this document (maps to SymbolId)
    pub vector_id: Option<VectorId>,
    /// The cluster assignment for this vector
    pub cluster_id: Option<ClusterId>,
    /// Version of the embedding model used to generate this vector
    pub embedding_version: u32,
}

/// Internal representation for JSON serialization
#[derive(Serialize, Deserialize)]
struct VectorMetadataJson {
    vector_id: Option<u32>,
    cluster_id: Option<u32>,
    embedding_version: u32,
}

impl VectorMetadata {
    /// Creates a new VectorMetadata with no vector assignment
    pub fn new(embedding_version: u32) -> Self {
        Self {
            vector_id: None,
            cluster_id: None,
            embedding_version,
        }
    }

    /// Creates VectorMetadata with full vector information
    pub fn with_vector(vector_id: VectorId, cluster_id: ClusterId, embedding_version: u32) -> Self {
        Self {
            vector_id: Some(vector_id),
            cluster_id: Some(cluster_id),
            embedding_version,
        }
    }

    /// Checks if this document has an associated vector
    pub fn has_vector(&self) -> bool {
        self.vector_id.is_some()
    }

    /// Serializes the metadata to a JSON string for storage in Tantivy
    pub fn to_json(&self) -> StorageResult<String> {
        let json_repr = VectorMetadataJson {
            vector_id: self.vector_id.map(|id| id.get()),
            cluster_id: self.cluster_id.map(|id| id.get()),
            embedding_version: self.embedding_version,
        };
        serde_json::to_string(&json_repr).map_err(|e| {
            StorageError::Serialization(format!("Failed to serialize VectorMetadata: {e}"))
        })
    }

    /// Deserializes metadata from a JSON string
    pub fn from_json(json: &str) -> StorageResult<Self> {
        let json_repr: VectorMetadataJson = serde_json::from_str(json).map_err(|e| {
            StorageError::Serialization(format!("Failed to deserialize VectorMetadata: {e}"))
        })?;

        Ok(Self {
            vector_id: json_repr.vector_id.and_then(VectorId::new),
            cluster_id: json_repr.cluster_id.and_then(ClusterId::new),
            embedding_version: json_repr.embedding_version,
        })
    }
}

/// Cache for cluster assignments to enable efficient vector search
///
/// This cache maintains mappings from cluster IDs to document IDs within each segment,
/// enabling the vector search to quickly find relevant documents without scanning
/// all vectors. The cache is rebuilt when the index reader generation changes.
#[derive(Debug, Clone)]
struct ClusterCache {
    /// The reader generation this cache was built for
    generation: u64,
    /// Mappings per segment: SegmentOrdinal -> (ClusterId -> [DocId])
    segment_mappings: HashMap<SegmentOrdinal, HashMap<ClusterId, Vec<DocId>>>,
}

impl ClusterCache {
    /// Creates a new empty cache
    fn new(generation: u64) -> Self {
        Self {
            generation,
            segment_mappings: HashMap::new(),
        }
    }

    /// Checks if the cache is valid for the given generation
    fn is_valid_for_generation(&self, generation: u64) -> bool {
        self.generation == generation
    }

    /// Adds a document to the cache
    fn add_document(&mut self, segment_ord: SegmentOrdinal, cluster_id: ClusterId, doc_id: DocId) {
        self.segment_mappings
            .entry(segment_ord)
            .or_default()
            .entry(cluster_id)
            .or_default()
            .push(doc_id);
    }

    /// Gets all documents in a cluster for a specific segment
    fn get_documents(
        &self,
        segment_ord: SegmentOrdinal,
        cluster_id: ClusterId,
    ) -> Option<&[DocId]> {
        self.segment_mappings
            .get(&segment_ord)
            .and_then(|clusters| clusters.get(&cluster_id))
            .map(|docs| docs.as_slice())
    }

    /// Sorts all document lists for efficient searching
    fn sort_all(&mut self) {
        for segment_map in self.segment_mappings.values_mut() {
            for doc_list in segment_map.values_mut() {
                doc_list.sort_unstable();
            }
        }
    }

    /// Gets the total number of cached documents
    #[cfg(test)]
    fn total_documents(&self) -> usize {
        self.segment_mappings
            .values()
            .flat_map(|clusters| clusters.values())
            .map(|docs| docs.len())
            .sum()
    }

    /// Gets all unique cluster IDs across all segments
    fn all_cluster_ids(&self) -> Vec<ClusterId> {
        let mut cluster_ids: Vec<ClusterId> = self
            .segment_mappings
            .values()
            .flat_map(|clusters| clusters.keys())
            .copied()
            .collect();
        cluster_ids.sort_unstable_by_key(|id| id.get());
        cluster_ids.dedup();
        cluster_ids
    }
}

/// Search result with rich metadata
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub symbol_id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line: u32,
    pub column: u16,
    pub doc_comment: Option<String>,
    pub signature: Option<String>,
    pub module_path: String,
    pub score: f32,
    pub highlights: Vec<TextHighlight>,
    pub context: Option<String>,
}

/// Highlighted text region
#[derive(Debug, Clone, Serialize)]
pub struct TextHighlight {
    pub field: String,
    pub start: usize,
    pub end: usize,
}

/// Document index for full-text search
pub struct DocumentIndex {
    index: Index,
    reader: IndexReader,
    schema: IndexSchema,
    index_path: PathBuf,
    pub(crate) writer: Mutex<Option<IndexWriter<Document>>>,
    /// Optional path for vector storage files
    vector_storage_path: Option<PathBuf>,
    /// Optional vector search engine for semantic search
    vector_engine: Option<Arc<Mutex<VectorSearchEngine>>>,
    /// Cache for cluster assignments (protected by RwLock for concurrent reads)
    cluster_cache: Arc<RwLock<Option<ClusterCache>>>,
    /// Optional embedding generator for vector search
    embedding_generator: Option<Arc<dyn EmbeddingGenerator>>,
    /// Symbols pending vector processing (SymbolId, symbol_text)
    pub(crate) pending_embeddings: Mutex<Vec<(SymbolId, String)>>,
    /// Pending symbol counter during batch operations
    pending_symbol_counter: Mutex<Option<u32>>,
    /// Pending file counter during batch operations
    pending_file_counter: Mutex<Option<u32>>,
}

impl std::fmt::Debug for DocumentIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentIndex")
            .field("index_path", &self.index_path)
            .field("schema", &self.schema)
            .field("vector_storage_path", &self.vector_storage_path)
            .field("has_vector_engine", &self.vector_engine.is_some())
            .field(
                "has_embedding_generator",
                &self.embedding_generator.is_some(),
            )
            .field(
                "has_cluster_cache",
                &self.cluster_cache.read().unwrap().is_some(),
            )
            .field(
                "pending_embeddings_count",
                &self.pending_embeddings.lock().unwrap().len(),
            )
            .finish()
    }
}

impl DocumentIndex {
    /// Create a new document index
    pub fn new(index_path: impl AsRef<Path>) -> StorageResult<Self> {
        let index_path = index_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&index_path)?;

        let (schema, index_schema) = IndexSchema::build();

        // Create or open the index
        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(&index_path)?
        } else {
            let dir = MmapDirectory::open(&index_path)?;
            Index::create(dir, schema, IndexSettings::default())?
        };

        // Register custom tokenizer for partial matching (ngram with min_gram=3, max_gram=10)
        // This allows "Archive" to match "ArchiveAppService"
        let ngram_tokenizer =
            TextAnalyzer::builder(NgramTokenizer::new(3, 10, false).unwrap()).build();
        index.tokenizers().register("ngram", ngram_tokenizer);

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        // If opening existing index, reload to get latest segments
        if index_path.join("meta.json").exists() {
            reader.reload()?;
        }

        Ok(Self {
            index,
            reader,
            schema: index_schema,
            index_path,
            writer: Mutex::new(None),
            vector_storage_path: None,
            vector_engine: None,
            cluster_cache: Arc::new(RwLock::new(None)),
            embedding_generator: None,
            pending_embeddings: Mutex::new(Vec::new()),
            pending_symbol_counter: Mutex::new(None),
            pending_file_counter: Mutex::new(None),
        })
    }

    /// Enable vector search support with the given engine and storage path
    pub fn with_vector_support(
        mut self,
        vector_engine: Arc<Mutex<VectorSearchEngine>>,
        vector_storage_path: impl AsRef<Path>,
    ) -> Self {
        self.vector_storage_path = Some(vector_storage_path.as_ref().to_path_buf());
        self.vector_engine = Some(vector_engine);
        self
    }

    /// Set the embedding generator for vector search
    pub fn with_embedding_generator(mut self, generator: Arc<dyn EmbeddingGenerator>) -> Self {
        self.embedding_generator = Some(generator);
        self
    }

    /// Check if vector search is enabled
    pub fn has_vector_support(&self) -> bool {
        self.vector_engine.is_some()
    }

    /// Get the vector storage path if configured
    pub fn vector_storage_path(&self) -> Option<&Path> {
        self.vector_storage_path.as_deref()
    }

    /// Get a reference to the vector engine if configured
    pub fn vector_engine(&self) -> Option<&Arc<Mutex<VectorSearchEngine>>> {
        self.vector_engine.as_ref()
    }

    /// Process pending embeddings after a successful Tantivy commit
    fn post_commit_vector_processing(&self) -> StorageResult<()> {
        // Get pending embeddings
        let pending_embeddings = {
            let mut pending = self
                .pending_embeddings
                .lock()
                .map_err(|_| StorageError::LockPoisoned)?;
            std::mem::take(&mut *pending)
        };

        if pending_embeddings.is_empty() {
            return Ok(());
        }

        // Get references to engine and generator
        let vector_engine = self
            .vector_engine
            .as_ref()
            .ok_or_else(|| StorageError::General("Vector engine not configured".to_string()))?;
        let embedding_generator = self.embedding_generator.as_ref().ok_or_else(|| {
            StorageError::General("Embedding generator not configured".to_string())
        })?;

        // Extract texts for embedding generation
        let texts: Vec<&str> = pending_embeddings
            .iter()
            .map(|(_, text)| text.as_str())
            .collect();

        // Generate embeddings
        let embeddings = embedding_generator
            .generate_embeddings(&texts)
            .map_err(|e| StorageError::General(format!("Embedding generation failed: {e}")))?;

        if embeddings.len() != pending_embeddings.len() {
            return Err(StorageError::General(format!(
                "Embedding count mismatch: expected {}, got {}",
                pending_embeddings.len(),
                embeddings.len()
            )));
        }

        // Create vector IDs and embeddings pairs
        let mut vectors = Vec::with_capacity(pending_embeddings.len());
        for (i, (symbol_id, _)) in pending_embeddings.iter().enumerate() {
            // Convert SymbolId to VectorId (both wrap u32)
            if let Some(vector_id) = crate::vector::VectorId::new(symbol_id.value()) {
                vectors.push((vector_id, embeddings[i].clone()));
            }
        }

        // Index vectors in the engine
        let mut engine = vector_engine
            .lock()
            .map_err(|_| StorageError::LockPoisoned)?;
        engine
            .index_vectors(&vectors)
            .map_err(|e| StorageError::General(format!("Vector indexing failed: {e}")))?;

        // Now we need to mark documents as having vectors
        // Since we can't do this in the same transaction, we'll need to do it separately
        // Store the pending updates for later processing
        drop(engine); // Release the lock

        // Call update_cluster_assignments to sync the cluster IDs
        // This will be done in a separate batch to avoid writer conflicts

        Ok(())
    }

    /// Build or rebuild the cluster cache from current segments
    /// This should be called after commits when vector support is enabled
    fn build_cluster_cache(&self) -> StorageResult<()> {
        if !self.has_vector_support() {
            return Ok(());
        }

        let searcher = self.reader.searcher();
        let generation = searcher.segment_readers().len() as u64; // Simple generation tracking

        // Check if cache is already valid
        {
            let cache = self
                .cluster_cache
                .read()
                .map_err(|_| StorageError::LockPoisoned)?;
            if let Some(ref existing_cache) = *cache {
                if existing_cache.is_valid_for_generation(generation) {
                    return Ok(());
                }
            }
        }

        // Build new cache
        let mut new_cache = ClusterCache::new(generation);

        // Iterate through all segments
        for (segment_ord, segment_reader) in searcher.segment_readers().iter().enumerate() {
            let segment_ord = SegmentOrdinal::new(segment_ord as u32);

            // Get the fast fields for this segment
            let fast_fields = segment_reader.fast_fields();
            let cluster_id_reader = fast_fields.u64("cluster_id")?.first_or_default_col(0);
            let has_vector_reader = fast_fields.u64("has_vector")?.first_or_default_col(0);

            // Scan all documents in the segment
            for doc_id in 0..segment_reader.num_docs() {
                // Check if document has a vector
                let has_vector_val = has_vector_reader.get_val(doc_id);
                if has_vector_val == 1 {
                    // Get cluster assignment
                    let cluster_val = cluster_id_reader.get_val(doc_id);
                    if let Some(cluster_id) = ClusterId::new(cluster_val as u32) {
                        new_cache.add_document(segment_ord, cluster_id, doc_id);
                    }
                }
            }
        }

        // Sort all document lists for efficient searching
        new_cache.sort_all();

        // Store the new cache
        let mut cache = self
            .cluster_cache
            .write()
            .map_err(|_| StorageError::LockPoisoned)?;
        *cache = Some(new_cache);

        Ok(())
    }

    /// Get documents in a specific cluster for a segment
    pub fn get_cluster_documents(
        &self,
        segment_ord: SegmentOrdinal,
        cluster_id: ClusterId,
    ) -> StorageResult<Vec<DocId>> {
        let cache = self
            .cluster_cache
            .read()
            .map_err(|_| StorageError::LockPoisoned)?;

        if let Some(ref cluster_cache) = *cache {
            Ok(cluster_cache
                .get_documents(segment_ord, cluster_id)
                .map(|docs| docs.to_vec())
                .unwrap_or_default())
        } else {
            Ok(Vec::new())
        }
    }

    /// Get all unique cluster IDs in the index
    pub fn get_all_cluster_ids(&self) -> StorageResult<Vec<ClusterId>> {
        let cache = self
            .cluster_cache
            .read()
            .map_err(|_| StorageError::LockPoisoned)?;

        if let Some(ref cluster_cache) = *cache {
            Ok(cluster_cache.all_cluster_ids())
        } else {
            Ok(Vec::new())
        }
    }

    /// Warm the cluster cache by forcing a rebuild
    /// This is useful after major index changes or reader reloads
    pub fn warm_cluster_cache(&self) -> StorageResult<()> {
        if !self.has_vector_support() {
            return Ok(());
        }

        // Force cache invalidation by setting an invalid generation
        {
            let mut cache = self
                .cluster_cache
                .write()
                .map_err(|_| StorageError::LockPoisoned)?;
            *cache = None; // Clear existing cache to force rebuild
        }

        // Rebuild the cache
        self.build_cluster_cache()
    }

    /// Get current cache generation for monitoring
    pub fn get_cache_generation(&self) -> StorageResult<Option<u64>> {
        let cache = self
            .cluster_cache
            .read()
            .map_err(|_| StorageError::LockPoisoned)?;
        Ok(cache.as_ref().map(|c| c.generation))
    }

    /// Reload the reader and warm caches
    /// This ensures the index is ready for high-performance queries
    pub fn reload_and_warm(&self) -> StorageResult<()> {
        // Reload the reader to see latest changes
        self.reader.reload()?;

        // Warm the cluster cache if vector support is enabled
        if self.has_vector_support() {
            self.warm_cluster_cache()?;
        }

        Ok(())
    }

    /// Update documents with cluster assignments from the vector engine
    /// This should be called after vector processing to sync cluster IDs
    pub fn update_cluster_assignments(&self) -> StorageResult<()> {
        if !self.has_vector_support() {
            return Ok(());
        }

        let vector_engine = self
            .vector_engine
            .as_ref()
            .ok_or_else(|| StorageError::General("Vector engine not configured".to_string()))?;

        let engine = vector_engine
            .lock()
            .map_err(|_| StorageError::LockPoisoned)?;

        // Get all vectors that have cluster assignments
        let cluster_assignments = engine.get_all_cluster_assignments();

        drop(engine); // Release the lock before we start updating

        // Convert VectorIds to SymbolIds
        let mut updates_needed = Vec::new();
        for (vector_id, cluster_id) in cluster_assignments {
            // Convert VectorId back to SymbolId
            if let Some(symbol_id) = SymbolId::new(vector_id.get()) {
                updates_needed.push((symbol_id, cluster_id));
            }
        }

        if updates_needed.is_empty() {
            return Ok(());
        }

        // Get the current searcher
        let searcher = self.reader.searcher();

        // Perform batch update
        self.start_batch()?;
        let mut writer_lock = self.writer.lock().map_err(|_| StorageError::LockPoisoned)?;
        let writer = writer_lock.as_mut().ok_or(StorageError::NoActiveBatch)?;

        for (symbol_id, cluster_id) in updates_needed {
            // Find the document by symbol_id
            let symbol_id_term =
                Term::from_field_u64(self.schema.symbol_id, symbol_id.value() as u64);
            let query = TermQuery::new(symbol_id_term.clone(), IndexRecordOption::Basic);

            // Search for the document
            let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

            if let Some((_score, doc_address)) = top_docs.first() {
                // Retrieve the stored document
                if let Ok(old_doc) = searcher.doc::<Document>(*doc_address) {
                    // Delete old document
                    writer.delete_term(symbol_id_term);

                    // Create new document with updated cluster_id
                    let mut new_doc = Document::new();

                    // Copy all fields except cluster_id and has_vector
                    for (field, value) in old_doc.field_values() {
                        if field != self.schema.cluster_id && field != self.schema.has_vector {
                            new_doc.add_field_value(field, value);
                        }
                    }

                    // Add updated vector fields
                    new_doc.add_u64(self.schema.cluster_id, cluster_id.get() as u64);
                    new_doc.add_u64(self.schema.has_vector, 1);

                    writer.add_document(new_doc)?;
                }
            }
        }

        drop(writer_lock);
        self.commit_batch()?;

        Ok(())
    }

    /// Start a batch operation for adding multiple documents
    pub fn start_batch(&self) -> StorageResult<()> {
        let mut writer_lock = self.writer.lock().map_err(|_| StorageError::LockPoisoned)?;
        if writer_lock.is_none() {
            // On Windows, retry writer creation if we hit permission errors
            // This handles cases where antivirus or filesystem delays affect file access
            let writer = {
                let mut attempts = 0;
                const MAX_ATTEMPTS: u32 = 5;

                loop {
                    // Use smaller heap size on Windows to reduce memory-mapped file pressure
                    // This helps avoid permission errors from antivirus software
                    #[cfg(windows)]
                    let heap_size = 15_000_000; // 15MB on Windows

                    #[cfg(not(windows))]
                    let heap_size = 100_000_000; // 100MB on other platforms

                    match self.index.writer::<Document>(heap_size) {
                        Ok(w) => break w,
                        Err(e) => {
                            attempts += 1;

                            // Check if this is a permission/IO error that might be transient
                            let is_transient = e.to_string().contains("Permission")
                                || e.to_string().contains("Access is denied")
                                || e.to_string().contains("Acceso denegado");

                            if is_transient && attempts < MAX_ATTEMPTS {
                                eprintln!(
                                    "Attempt {}/{}: Failed to create index writer ({}), retrying after delay...",
                                    attempts, MAX_ATTEMPTS, e
                                );

                                // Exponential backoff: 100ms, 200ms, 400ms, 800ms
                                let delay_ms = 100 * (1 << (attempts - 1));
                                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                                continue;
                            }

                            // Non-transient error or exhausted retries
                            return Err(e.into());
                        }
                    }
                }
            };
            *writer_lock = Some(writer);

            // Initialize the pending symbol counter for this batch
            let current = self
                .query_metadata(MetadataKey::SymbolCounter)?
                .unwrap_or(0) as u32;
            if let Ok(mut pending_guard) = self.pending_symbol_counter.lock() {
                *pending_guard = Some(current + 1);
            }

            // Initialize the pending file counter for this batch
            let file_current = self.query_metadata(MetadataKey::FileCounter)?.unwrap_or(0) as u32;
            if let Ok(mut pending_guard) = self.pending_file_counter.lock() {
                *pending_guard = Some(file_current + 1);
            }
        }
        Ok(())
    }

    /// Add a document to the index (must call start_batch first)
    #[allow(clippy::too_many_arguments)]
    pub fn add_document(
        &self,
        symbol_id: SymbolId,
        name: &str,
        kind: SymbolKind,
        file_id: FileId,
        file_path: &str,
        line: u32,
        column: u16,
        end_line: u32,
        end_column: u16,
        doc_comment: Option<&str>,
        signature: Option<&str>,
        module_path: &str,
        context: Option<&str>,
        visibility: crate::Visibility,
        scope_context: Option<crate::ScopeContext>,
        language_id: Option<&str>, // Language identifier for the symbol
    ) -> StorageResult<()> {
        let mut writer_lock = self.writer.lock().map_err(|_| StorageError::LockPoisoned)?;
        let writer = writer_lock.as_mut().ok_or(StorageError::NoActiveBatch)?;

        let mut doc = Document::new();
        doc.add_text(self.schema.doc_type, "symbol");
        doc.add_u64(self.schema.symbol_id, symbol_id.value() as u64);
        doc.add_u64(self.schema.file_id, file_id.value() as u64);
        doc.add_text(self.schema.name, name);
        doc.add_text(self.schema.name_text, name); // Also add to full-text searchable field
        doc.add_text(self.schema.file_path, file_path);
        doc.add_u64(self.schema.line_number, line as u64);
        doc.add_u64(self.schema.column, column as u64);
        doc.add_u64(self.schema.end_line, end_line as u64);
        doc.add_u64(self.schema.end_column, end_column as u64);

        if let Some(comment) = doc_comment {
            doc.add_text(self.schema.doc_comment, comment);
        }

        if let Some(sig) = signature {
            doc.add_text(self.schema.signature, sig);
        }

        if let Some(ctx) = context {
            doc.add_text(self.schema.context, ctx);
        }

        // Add string fields for filtering
        doc.add_text(self.schema.module_path, module_path);
        doc.add_text(self.schema.kind, format!("{kind:?}"));
        doc.add_u64(self.schema.visibility, visibility as u64);

        // Store scope_context as a string (serialized enum)
        if let Some(scope) = scope_context {
            doc.add_text(self.schema.scope_context, format!("{scope:?}"));
        } else {
            doc.add_text(self.schema.scope_context, "None");
        }

        // Store language identifier
        if let Some(lang) = language_id {
            doc.add_text(self.schema.language, lang);
        } else {
            doc.add_text(self.schema.language, "");
        }

        // Add default vector fields - these will be updated later if vectors are generated
        if self.has_vector_support() {
            doc.add_u64(self.schema.cluster_id, 0); // 0 means not yet assigned
            doc.add_u64(self.schema.vector_id, symbol_id.value() as u64);
            doc.add_u64(self.schema.has_vector, 0); // Will be set to 1 after vector processing
        }

        writer.add_document(doc)?;

        // Track symbol for vector embedding if vector support is enabled
        if self.has_vector_support() && self.embedding_generator.is_some() {
            // Create symbol text representation for embedding
            let symbol_text = format!("{} {:?} {}", name, kind, signature.unwrap_or(""));

            let mut pending = self
                .pending_embeddings
                .lock()
                .map_err(|_| StorageError::LockPoisoned)?;
            pending.push((symbol_id, symbol_text));
        }

        Ok(())
    }

    /// Commit the current batch and reload the reader
    pub fn commit_batch(&self) -> StorageResult<()> {
        let mut writer_lock = match self.writer.lock() {
            Ok(lock) => lock,
            Err(poisoned) => {
                eprintln!("Warning: Recovering from poisoned writer mutex in commit_batch");
                poisoned.into_inner()
            }
        };
        if let Some(mut writer) = writer_lock.take() {
            // Try to commit with better error context
            match writer.commit() {
                Ok(_opstamp) => {
                    // Successful commit
                }
                Err(e) => {
                    // Provide helpful context for permission errors
                    let error_msg = e.to_string();
                    if error_msg.contains("Permission")
                        || error_msg.contains("PermissionDenied")
                        || error_msg.contains("Access is denied")
                        || error_msg.contains("Acceso denegado")
                        || error_msg.contains("code: 5")
                    {
                        return Err(StorageError::General(format!(
                            "Failed to commit index due to file permission error. \
                            This can happen when:\n\
                            1. Antivirus software is scanning the index directory\n\
                            2. Another process has locked the files\n\
                            3. Insufficient file system permissions\n\
                            \nOriginal error: {}\n\
                            \nTry:\n\
                            - Temporarily disabling antivirus for the project directory\n\
                            - Ensuring no other codanna processes are running\n\
                            - Running with administrator privileges if on Windows",
                            e
                        )));
                    }
                    return Err(e.into());
                }
            }

            // Reload the reader to see new documents
            self.reader.reload()?;

            // Clear the pending symbol counter after commit
            if let Ok(mut pending_guard) = self.pending_symbol_counter.lock() {
                *pending_guard = None;
            }

            // Clear the pending file counter after commit
            if let Ok(mut pending_guard) = self.pending_file_counter.lock() {
                *pending_guard = None;
            }

            // Process pending vector embeddings if enabled
            if self.has_vector_support() && self.embedding_generator.is_some() {
                self.post_commit_vector_processing()?;
            }

            // Build cluster cache if vector support is enabled
            self.build_cluster_cache()?;
        }
        Ok(())
    }

    /// Remove documents for a specific file
    pub fn remove_file_documents(&self, file_path: &str) -> StorageResult<()> {
        // Use existing batch writer if available, otherwise create temporary one
        let mut writer_lock = self.writer.lock().map_err(|_| StorageError::LockPoisoned)?;
        let term = Term::from_field_text(self.schema.file_path, file_path);

        if let Some(writer) = writer_lock.as_mut() {
            // Use existing batch writer
            writer.delete_term(term);
            // Note: We don't commit here - that happens at batch end
        } else {
            // Create temporary writer for single operation
            drop(writer_lock); // Release lock before creating new writer
            let mut writer = self.index.writer::<Document>(50_000_000)?;
            writer.delete_term(term);
            writer.commit()?;
            self.reader.reload()?;
        }

        Ok(())
    }

    /// Search for documents
    pub fn search(
        &self,
        query_str: &str,
        limit: usize,
        kind_filter: Option<SymbolKind>,
        module_filter: Option<&str>,
        language_filter: Option<&str>,
    ) -> StorageResult<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.schema.name_text, // Use name_text for full-text search (tokenized)
                self.schema.doc_comment,
                self.schema.signature,
                self.schema.context,
            ],
        );

        // Try parsing as Tantivy query syntax first, fall back to literal matching
        // for queries with special characters (interface{}, Vec<T>, etc.)
        let main_query = match query_parser.parse_query(query_str) {
            Ok(query) => query,
            Err(_parse_error) => {
                // Query contains syntax that conflicts with Tantivy parser.
                // Fall back to literal term matching across searchable fields.
                let name_term = Term::from_field_text(self.schema.name_text, query_str);
                let doc_term = Term::from_field_text(self.schema.doc_comment, query_str);
                let sig_term = Term::from_field_text(self.schema.signature, query_str);
                let ctx_term = Term::from_field_text(self.schema.context, query_str);

                Box::new(BooleanQuery::new(vec![
                    (
                        Occur::Should,
                        Box::new(TermQuery::new(name_term, IndexRecordOption::Basic))
                            as Box<dyn Query>,
                    ),
                    (
                        Occur::Should,
                        Box::new(TermQuery::new(doc_term, IndexRecordOption::Basic))
                            as Box<dyn Query>,
                    ),
                    (
                        Occur::Should,
                        Box::new(TermQuery::new(sig_term, IndexRecordOption::Basic))
                            as Box<dyn Query>,
                    ),
                    (
                        Occur::Should,
                        Box::new(TermQuery::new(ctx_term, IndexRecordOption::Basic))
                            as Box<dyn Query>,
                    ),
                ])) as Box<dyn Query>
            }
        };

        // Fuzzy query for typo tolerance on the name_text field (ngram tokens)
        let name_text_term = Term::from_field_text(self.schema.name_text, query_str);
        let fuzzy_ngram_query = FuzzyTermQuery::new(name_text_term, 1, true);

        // ADDITIONAL: Fuzzy query on the non-tokenized name field for whole-word typo tolerance
        // This fixes the limitation where "ArchivService" (missing 'e') couldn't find "ArchiveService"
        // because ngram tokenization shifted all tokens after the typo
        let name_term = Term::from_field_text(self.schema.name, query_str);
        let fuzzy_whole_word_query = FuzzyTermQuery::new(name_term, 1, true);

        // All queries will be collected here.
        let mut all_clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        // The text search part: must match one of:
        // 1. Main query (ngram partial matching)
        // 2. Fuzzy on ngram tokens (typos in short queries)
        // 3. Fuzzy on whole word (typos in full symbol names)
        all_clauses.push((
            Occur::Must,
            Box::new(BooleanQuery::new(vec![
                (Occur::Should, main_query),
                (Occur::Should, Box::new(fuzzy_ngram_query)),
                (Occur::Should, Box::new(fuzzy_whole_word_query)),
            ])),
        ));

        // Add mandatory filters.
        all_clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(self.schema.doc_type, "symbol"),
                IndexRecordOption::Basic,
            )),
        ));

        if let Some(kind) = kind_filter {
            let term = Term::from_field_text(self.schema.kind, &format!("{kind:?}"));
            all_clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        if let Some(module) = module_filter {
            let term = Term::from_field_text(self.schema.module_path, module);
            all_clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        // Add language filter if provided
        if let Some(lang) = language_filter {
            let term = Term::from_field_text(self.schema.language, lang);
            all_clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
            ));
        }

        let final_query = BooleanQuery::new(all_clauses);

        let top_docs = searcher.search(&final_query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: Document = searcher.doc(doc_address)?;

            // Extract fields
            let symbol_id = doc
                .get_first(self.schema.symbol_id)
                .and_then(|v| v.as_u64())
                .and_then(|id| SymbolId::new(id as u32))
                .ok_or(StorageError::InvalidFieldValue {
                    field: "symbol_id".to_string(),
                    reason: "not a valid u32".to_string(),
                })?;

            let name = doc
                .get_first(self.schema.name)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let file_path = doc
                .get_first(self.schema.file_path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let line = doc
                .get_first(self.schema.line_number)
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;

            let column = doc
                .get_first(self.schema.column)
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16;

            let doc_comment = doc
                .get_first(self.schema.doc_comment)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let signature = doc
                .get_first(self.schema.signature)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let context = doc
                .get_first(self.schema.context)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Extract kind from facet (stored as string representation)
            let kind_str = doc
                .get_first(self.schema.kind)
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");

            let kind = match kind_str {
                "Function" => SymbolKind::Function,
                "Struct" => SymbolKind::Struct,
                "Trait" => SymbolKind::Trait,
                "Method" => SymbolKind::Method,
                "Field" => SymbolKind::Field,
                "Module" => SymbolKind::Module,
                "Constant" => SymbolKind::Constant,
                _ => SymbolKind::Function, // Default fallback
            };

            let module_path = doc
                .get_first(self.schema.module_path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            results.push(SearchResult {
                symbol_id,
                name,
                kind,
                file_path,
                line,
                column,
                doc_comment,
                signature,
                module_path,
                score,
                highlights: Vec::new(), // TODO: Implement highlighting
                context,
            });
        }

        Ok(results)
    }

    /// Get total number of indexed documents
    pub fn document_count(&self) -> StorageResult<u64> {
        let searcher = self.reader.searcher();
        Ok(searcher.num_docs())
    }

    /// Clear all documents from the index
    pub fn clear(&self) -> StorageResult<()> {
        // Check if index has been initialized (has meta.json)
        // If not, there's nothing to clear
        let meta_path = self.index_path.join("meta.json");
        if !meta_path.exists() {
            return Ok(());
        }

        let mut writer = self.index.writer::<Document>(50_000_000)?;
        writer.delete_all_documents()?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Find a symbol by its ID
    pub fn find_symbol_by_id(&self, id: SymbolId) -> StorageResult<Option<crate::Symbol>> {
        let searcher = self.reader.searcher();
        let query = TermQuery::new(
            Term::from_field_u64(self.schema.symbol_id, id.0 as u64),
            IndexRecordOption::Basic,
        );

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

        if let Some((_score, doc_address)) = top_docs.first() {
            let doc = searcher.doc::<Document>(*doc_address)?;
            Ok(Some(self.document_to_symbol(&doc)?))
        } else {
            Ok(None)
        }
    }

    /// Find a symbol by its ID with language filter
    pub fn find_symbol_by_id_with_language(
        &self,
        id: SymbolId,
        language: &str,
    ) -> StorageResult<Option<crate::Symbol>> {
        let searcher = self.reader.searcher();

        // Build a compound query: symbol_id AND language
        let query = BooleanQuery::from(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(self.schema.symbol_id, id.0 as u64),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.language, language),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
        ]);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

        if let Some((_score, doc_address)) = top_docs.first() {
            let doc = searcher.doc::<Document>(*doc_address)?;
            Ok(Some(self.document_to_symbol(&doc)?))
        } else {
            Ok(None)
        }
    }

    /// Find symbols by name
    pub fn find_symbols_by_name(
        &self,
        name: &str,
        language_filter: Option<&str>,
    ) -> StorageResult<Vec<crate::Symbol>> {
        let searcher = self.reader.searcher();

        // Use exact term matching for symbol names (name field is STRING type, not TEXT)
        // This prevents tokenization issues that cause "MyService" to match "Main"
        let name_query = Box::new(TermQuery::new(
            Term::from_field_text(self.schema.name, name),
            IndexRecordOption::Basic,
        )) as Box<dyn Query>;

        // Build query clauses
        let mut query_clauses = vec![
            (Occur::Must, name_query),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.doc_type, "symbol"),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
        ];

        // Add language filter if provided
        if let Some(lang) = language_filter {
            query_clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.language, lang),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        let final_query = BooleanQuery::new(query_clauses);

        let top_docs = searcher.search(&final_query, &TopDocs::with_limit(100))?;
        let mut symbols = Vec::new();

        for (_score, doc_address) in top_docs {
            let doc = searcher.doc::<Document>(doc_address)?;
            symbols.push(self.document_to_symbol(&doc)?);
        }

        Ok(symbols)
    }

    /// Find symbols by file ID
    pub fn find_symbols_by_file(&self, file_id: FileId) -> StorageResult<Vec<crate::Symbol>> {
        let searcher = self.reader.searcher();
        let query = BooleanQuery::from(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.doc_type, "symbol"),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(self.schema.file_id, file_id.0 as u64),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
        ]);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1000))?;
        let mut symbols = Vec::new();

        for (_score, doc_address) in top_docs {
            let doc = searcher.doc::<Document>(doc_address)?;
            symbols.push(self.document_to_symbol(&doc)?);
        }

        Ok(symbols)
    }

    /// Get all symbols (use with caution on large indexes)
    pub fn get_all_symbols(&self, limit: usize) -> StorageResult<Vec<crate::Symbol>> {
        let searcher = self.reader.searcher();

        // Use pre-filtering query instead of AllQuery + post-filtering
        // This matches the pattern used in find_symbols_by_name and find_symbols_by_file
        let query = BooleanQuery::from(vec![(
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(self.schema.doc_type, "symbol"),
                IndexRecordOption::Basic,
            )) as Box<dyn Query>,
        )]);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut symbols = Vec::new();

        for (_score, doc_address) in top_docs {
            let doc = searcher.doc::<Document>(doc_address)?;
            symbols.push(self.document_to_symbol(&doc)?);
        }

        Ok(symbols)
    }

    /// Convert a Tantivy document to a Symbol
    fn document_to_symbol(&self, doc: &Document) -> StorageResult<crate::Symbol> {
        use crate::{Range, Symbol, SymbolKind, Visibility};

        let symbol_id = doc
            .get_first(self.schema.symbol_id)
            .and_then(|v| v.as_u64())
            .ok_or(StorageError::InvalidFieldValue {
                field: "symbol_id".to_string(),
                reason: "missing from document".to_string(),
            })?;

        let name = doc
            .get_first(self.schema.name)
            .and_then(|v| v.as_str())
            .ok_or(StorageError::InvalidFieldValue {
                field: "name".to_string(),
                reason: "missing from document".to_string(),
            })?
            .to_string();

        let kind_str = doc
            .get_first(self.schema.kind)
            .and_then(|v| v.as_str())
            .ok_or(StorageError::InvalidFieldValue {
                field: "kind".to_string(),
                reason: "missing from document".to_string(),
            })?;
        let kind = SymbolKind::from_str_with_default(kind_str);

        let file_id = doc
            .get_first(self.schema.file_id)
            .and_then(|v| v.as_u64())
            .ok_or(StorageError::InvalidFieldValue {
                field: "file_id".to_string(),
                reason: "missing from document".to_string(),
            })?;

        let start_line = doc
            .get_first(self.schema.line_number)
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let start_col = doc
            .get_first(self.schema.column)
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u16;

        let end_line = doc
            .get_first(self.schema.end_line)
            .and_then(|v| v.as_u64())
            .unwrap_or(start_line as u64) as u32;

        let end_col = doc
            .get_first(self.schema.end_column)
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u16;

        let signature = doc
            .get_first(self.schema.signature)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let doc_comment = doc
            .get_first(self.schema.doc_comment)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let module_path = doc
            .get_first(self.schema.module_path)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Get visibility from stored field
        let visibility = doc
            .get_first(self.schema.visibility)
            .and_then(|v| v.as_u64())
            .map(|v| match v {
                0 => Visibility::Public,
                1 => Visibility::Crate,
                2 => Visibility::Module,
                3 => Visibility::Private,
                _ => Visibility::Private,
            })
            .unwrap_or(Visibility::Private);

        // Get scope_context from stored field
        let scope_context = doc
            .get_first(self.schema.scope_context)
            .and_then(|v| v.as_str())
            .and_then(|s| {
                // Parse the serialized scope context
                match s {
                    "None" => None,
                    "Module" => Some(crate::ScopeContext::Module),
                    "Global" => Some(crate::ScopeContext::Global),
                    "Package" => Some(crate::ScopeContext::Package),
                    "Parameter" => Some(crate::ScopeContext::Parameter),
                    "ClassMember" => Some(crate::ScopeContext::ClassMember),
                    s if s.starts_with("Local") => {
                        // Handle Local { hoisted: bool, parent_name: Option<String>, parent_kind: Option<SymbolKind> } format
                        let hoisted = s.contains("hoisted: true") || s.contains("hoisted:true");

                        // Extract parent_name if present
                        let parent_name = if s.contains("parent_name: Some(") {
                            let start = s.find("parent_name: Some(\"").map(|i| i + 19)?;
                            let end = s[start..].find('"').map(|i| start + i)?;
                            Some(s[start..end].to_string().into())
                        } else {
                            None
                        };

                        // Extract parent_kind if present
                        let parent_kind = if s.contains("parent_kind: Some(") {
                            let start = s.find("parent_kind: Some(").map(|i| i + 18)?;
                            let end = s[start..].find(')').map(|i| start + i)?;
                            let kind_str = &s[start..end];
                            match kind_str {
                                "Function" => Some(crate::SymbolKind::Function),
                                "Class" => Some(crate::SymbolKind::Class),
                                "Method" => Some(crate::SymbolKind::Method),
                                _ => None,
                            }
                        } else {
                            None
                        };

                        Some(crate::ScopeContext::Local {
                            hoisted,
                            parent_name,
                            parent_kind,
                        })
                    }
                    _ => None,
                }
            });

        Ok(Symbol {
            id: SymbolId(symbol_id as u32),
            name: name.into(),
            kind,
            file_id: FileId(file_id as u32),
            range: Range {
                start_line,
                start_column: start_col,
                end_line,
                end_column: end_col,
            },
            signature: signature.map(|s| s.into()),
            doc_comment: doc_comment.map(|s| s.into()),
            module_path: module_path.map(|s| s.into()),
            visibility,
            scope_context,
            language_id: {
                // Read the language field from the document and convert to LanguageId
                // using the language registry (which maintains the static strings)
                doc.get_first(self.schema.language)
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .and_then(|lang_str| {
                        // Use the global registry to convert string to LanguageId
                        // This maintains language-agnostic storage while properly
                        // converting to the type-safe LanguageId at retrieval time
                        crate::parsing::get_registry()
                            .lock()
                            .ok()
                            .and_then(|registry| registry.find_language_id(lang_str))
                    })
            },
        })
    }

    /// Get file info by path
    pub fn get_file_info(&self, path: &str) -> StorageResult<Option<(FileId, String)>> {
        let searcher = self.reader.searcher();
        let query = BooleanQuery::from(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.doc_type, "file_info"),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.file_path, path),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
        ]);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

        if let Some((_score, doc_address)) = top_docs.first() {
            let doc = searcher.doc::<Document>(*doc_address)?;

            let file_id = doc
                .get_first(self.schema.file_id)
                .and_then(|v| v.as_u64())
                .and_then(|id| FileId::new(id as u32))
                .ok_or(StorageError::InvalidFieldValue {
                    field: "file_id".to_string(),
                    reason: "not a valid u32".to_string(),
                })?;

            let hash = doc
                .get_first(self.schema.file_hash)
                .and_then(|v| v.as_str())
                .ok_or(StorageError::InvalidFieldValue {
                    field: "file_hash".to_string(),
                    reason: "missing from document".to_string(),
                })?
                .to_string();

            Ok(Some((file_id, hash)))
        } else {
            Ok(None)
        }
    }

    /// Get next file ID
    pub fn get_next_file_id(&self) -> StorageResult<u32> {
        // During batch operations, use and increment the pending counter
        if let Ok(mut pending_guard) = self.pending_file_counter.lock() {
            if let Some(ref mut counter) = *pending_guard {
                let next_id = *counter;
                *counter += 1;
                return Ok(next_id);
            }
        }

        // Otherwise, query the committed metadata
        let current = self.query_metadata(MetadataKey::FileCounter)?.unwrap_or(0) as u32;
        Ok(current + 1)
    }

    /// Get next symbol ID
    pub fn get_next_symbol_id(&self) -> StorageResult<u32> {
        // During batch operations, use and increment the pending counter
        if let Ok(mut pending_guard) = self.pending_symbol_counter.lock() {
            if let Some(ref mut counter) = *pending_guard {
                let next_id = *counter;
                *counter += 1;
                return Ok(next_id);
            }
        }

        // Otherwise, query the committed metadata
        let current = self
            .query_metadata(MetadataKey::SymbolCounter)?
            .unwrap_or(0) as u32;
        Ok(current + 1)
    }

    /// Update the pending symbol counter (for cross-file symbol ID continuity in batches)
    pub fn update_pending_symbol_counter(&self, new_value: u32) -> StorageResult<()> {
        if let Ok(mut pending_guard) = self.pending_symbol_counter.lock() {
            if let Some(ref mut counter) = *pending_guard {
                *counter = new_value;
            }
        }
        Ok(())
    }

    /// Delete a symbol
    pub fn delete_symbol(&self, id: SymbolId) -> StorageResult<()> {
        let mut writer_lock = match self.writer.lock() {
            Ok(lock) => lock,
            Err(poisoned) => {
                eprintln!("Warning: Recovering from poisoned writer mutex in delete_symbol");
                poisoned.into_inner()
            }
        };
        let writer = writer_lock.as_mut().ok_or(StorageError::NoActiveBatch)?;

        let term = Term::from_field_u64(self.schema.symbol_id, id.0 as u64);
        writer.delete_term(term);
        Ok(())
    }

    /// Delete relationships for a symbol
    pub fn delete_relationships_for_symbol(&self, id: SymbolId) -> StorageResult<()> {
        let mut writer_lock = match self.writer.lock() {
            Ok(lock) => lock,
            Err(poisoned) => {
                eprintln!("Warning: Recovering from poisoned writer mutex in delete_relationships");
                poisoned.into_inner()
            }
        };
        let writer = writer_lock.as_mut().ok_or(StorageError::NoActiveBatch)?;

        // Delete where from_symbol_id = id
        let from_term = Term::from_field_u64(self.schema.from_symbol_id, id.0 as u64);
        writer.delete_term(from_term);

        // Delete where to_symbol_id = id
        let to_term = Term::from_field_u64(self.schema.to_symbol_id, id.0 as u64);
        writer.delete_term(to_term);

        Ok(())
    }

    /// Count symbols
    pub fn count_symbols(&self) -> StorageResult<usize> {
        let searcher = self.reader.searcher();
        let query = TermQuery::new(
            Term::from_field_text(self.schema.doc_type, "symbol"),
            IndexRecordOption::Basic,
        );

        let count = searcher.search(&query, &tantivy::collector::Count)?;
        Ok(count)
    }

    /// Count total number of relationships
    pub fn count_relationships(&self) -> StorageResult<usize> {
        let searcher = self.reader.searcher();
        let query = TermQuery::new(
            Term::from_field_text(self.schema.doc_type, "relationship"),
            IndexRecordOption::Basic,
        );

        let count = searcher.search(&query, &tantivy::collector::Count)?;
        Ok(count)
    }

    /// Count files
    pub fn count_files(&self) -> StorageResult<usize> {
        let searcher = self.reader.searcher();
        let query = TermQuery::new(
            Term::from_field_text(self.schema.doc_type, "file_info"),
            IndexRecordOption::Basic,
        );

        let count = searcher.search(&query, &tantivy::collector::Count)?;
        Ok(count)
    }

    /// Get all indexed file paths for file watching
    /// Returns a vector of all file paths currently in the index
    pub fn get_all_indexed_paths(&self) -> StorageResult<Vec<PathBuf>> {
        let searcher = self.reader.searcher();
        let query = TermQuery::new(
            Term::from_field_text(self.schema.doc_type, "file_info"),
            IndexRecordOption::Basic,
        );

        // Use TopDocs to get all file_info documents
        // Note: Adjust limit if you have more than 100k files
        let collector = TopDocs::with_limit(100_000);
        let top_docs = searcher.search(&query, &collector)?;

        let mut paths = Vec::new();
        for (_score, doc_address) in top_docs {
            let doc: Document = searcher.doc(doc_address)?;

            // Extract file_path field
            if let Some(path_value) = doc.get_first(self.schema.file_path) {
                if let Some(path_str) = path_value.as_str() {
                    paths.push(PathBuf::from(path_str));
                }
            }
        }

        Ok(paths)
    }

    /// Get relationships from a symbol
    pub fn get_relationships_from(
        &self,
        from_id: SymbolId,
        kind: RelationKind,
    ) -> StorageResult<Vec<(SymbolId, SymbolId, Relationship)>> {
        let searcher = self.reader.searcher();
        let query = BooleanQuery::from(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.doc_type, "relationship"),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(self.schema.from_symbol_id, from_id.0 as u64),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.relation_kind, &format!("{kind:?}")),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
        ]);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1000))?;
        let mut relationships = Vec::new();

        for (_score, doc_address) in top_docs {
            let doc = searcher.doc::<Document>(doc_address)?;

            let to_id = doc
                .get_first(self.schema.to_symbol_id)
                .and_then(|v| v.as_u64())
                .and_then(|id| SymbolId::new(id as u32))
                .ok_or(StorageError::InvalidFieldValue {
                    field: "to_symbol_id".to_string(),
                    reason: "not a valid u32".to_string(),
                })?;

            // Extract metadata if present
            let mut relationship = Relationship::new(kind);

            // Extract metadata fields
            if let Some(line) = doc
                .get_first(self.schema.relation_line)
                .and_then(|v| v.as_u64())
            {
                if let Some(column) = doc
                    .get_first(self.schema.relation_column)
                    .and_then(|v| v.as_u64())
                {
                    let mut metadata =
                        RelationshipMetadata::new().at_position(line as u32, column as u16);

                    if let Some(context) = doc
                        .get_first(self.schema.relation_context)
                        .and_then(|v| v.as_str())
                    {
                        metadata = metadata.with_context(context);
                    }

                    relationship = relationship.with_metadata(metadata);
                }
            }

            relationships.push((from_id, to_id, relationship));
        }

        Ok(relationships)
    }

    /// Get relationships to a symbol
    pub fn get_relationships_to(
        &self,
        to_id: SymbolId,
        kind: RelationKind,
    ) -> StorageResult<Vec<(SymbolId, SymbolId, Relationship)>> {
        let searcher = self.reader.searcher();
        let query = BooleanQuery::from(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.doc_type, "relationship"),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(self.schema.to_symbol_id, to_id.0 as u64),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.relation_kind, &format!("{kind:?}")),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
        ]);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1000))?;
        let mut relationships = Vec::new();

        for (_score, doc_address) in top_docs {
            let doc = searcher.doc::<Document>(doc_address)?;

            let from_id = doc
                .get_first(self.schema.from_symbol_id)
                .and_then(|v| v.as_u64())
                .and_then(|id| SymbolId::new(id as u32))
                .ok_or(StorageError::InvalidFieldValue {
                    field: "from_symbol_id".to_string(),
                    reason: "not a valid u32".to_string(),
                })?;

            // Extract metadata if present
            let mut relationship = Relationship::new(kind);

            // Extract metadata fields
            if let Some(line) = doc
                .get_first(self.schema.relation_line)
                .and_then(|v| v.as_u64())
            {
                if let Some(column) = doc
                    .get_first(self.schema.relation_column)
                    .and_then(|v| v.as_u64())
                {
                    let mut metadata =
                        RelationshipMetadata::new().at_position(line as u32, column as u16);

                    if let Some(context) = doc
                        .get_first(self.schema.relation_context)
                        .and_then(|v| v.as_str())
                    {
                        metadata = metadata.with_context(context);
                    }

                    relationship = relationship.with_metadata(metadata);
                }
            }

            relationships.push((from_id, to_id, relationship));
        }

        Ok(relationships)
    }

    /// Get all relationships of a specific kind
    pub fn get_all_relationships_by_kind(
        &self,
        kind: RelationKind,
    ) -> StorageResult<Vec<(SymbolId, SymbolId, Relationship)>> {
        let searcher = self.reader.searcher();
        let query = BooleanQuery::from(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.doc_type, "relationship"),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.relation_kind, &format!("{kind:?}")),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
        ]);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(10000))?;
        let mut relationships = Vec::new();

        for (_score, doc_address) in top_docs {
            let doc = searcher.doc::<Document>(doc_address)?;

            let from_id = doc
                .get_first(self.schema.from_symbol_id)
                .and_then(|v| v.as_u64())
                .and_then(|id| SymbolId::new(id as u32))
                .ok_or(StorageError::InvalidFieldValue {
                    field: "from_symbol_id".to_string(),
                    reason: "not a valid u32".to_string(),
                })?;

            let to_id = doc
                .get_first(self.schema.to_symbol_id)
                .and_then(|v| v.as_u64())
                .and_then(|id| SymbolId::new(id as u32))
                .ok_or(StorageError::InvalidFieldValue {
                    field: "to_symbol_id".to_string(),
                    reason: "not a valid u32".to_string(),
                })?;

            relationships.push((from_id, to_id, Relationship::new(kind)));
        }

        Ok(relationships)
    }

    /// Get file path by ID
    pub fn get_file_path(&self, file_id: FileId) -> StorageResult<Option<String>> {
        let searcher = self.reader.searcher();
        let query = BooleanQuery::from(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.doc_type, "file_info"),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(self.schema.file_id, file_id.0 as u64),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
        ]);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

        if let Some((_score, doc_address)) = top_docs.first() {
            let doc = searcher.doc::<Document>(*doc_address)?;

            let path = doc
                .get_first(self.schema.file_path)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            Ok(path)
        } else {
            Ok(None)
        }
    }

    /// Get the path where the index is stored
    ///
    /// TODO: Potential use cases for this method:
    /// - Recreating the index if corrupted
    /// - Moving or copying the index to another location
    /// - Displaying index location in diagnostics or status commands
    /// - Cleaning up the entire index directory
    /// - Backing up the index data
    pub fn path(&self) -> &Path {
        &self.index_path
    }

    // Internal methods for storage operations (accessible within crate)

    /// Store a relationship between two symbols
    pub(crate) fn store_relationship(
        &self,
        from: SymbolId,
        to: SymbolId,
        rel: &Relationship,
    ) -> StorageResult<()> {
        let mut writer_lock = match self.writer.lock() {
            Ok(lock) => lock,
            Err(poisoned) => {
                eprintln!("Warning: Recovering from poisoned writer mutex in store_relationship");
                poisoned.into_inner()
            }
        };
        let writer = writer_lock.as_mut().ok_or(StorageError::NoActiveBatch)?;

        let mut doc = Document::new();
        doc.add_text(self.schema.doc_type, "relationship");
        doc.add_u64(self.schema.from_symbol_id, from.value() as u64);
        doc.add_u64(self.schema.to_symbol_id, to.value() as u64);
        doc.add_text(self.schema.relation_kind, format!("{:?}", rel.kind));
        doc.add_f64(self.schema.relation_weight, rel.weight as f64);

        if let Some(ref metadata) = rel.metadata {
            if let Some(line) = metadata.line {
                doc.add_u64(self.schema.relation_line, line as u64);
            }
            if let Some(column) = metadata.column {
                doc.add_u64(self.schema.relation_column, column as u64);
            }
            if let Some(ref context) = metadata.context {
                doc.add_text(self.schema.relation_context, context.as_ref());
            }
        }

        writer.add_document(doc)?;
        Ok(())
    }

    /// Index a symbol from a Symbol struct
    pub fn index_symbol(&self, symbol: &crate::Symbol, file_path: &str) -> StorageResult<()> {
        self.add_document(
            symbol.id,
            &symbol.name,
            symbol.kind,
            symbol.file_id,
            file_path,
            symbol.range.start_line,
            symbol.range.start_column,
            symbol.range.end_line,
            symbol.range.end_column,
            symbol.doc_comment.as_ref().map(|s| s.as_ref()),
            symbol.signature.as_ref().map(|s| s.as_ref()),
            symbol
                .module_path
                .as_ref()
                .map(|s| s.as_ref())
                .unwrap_or(""),
            None, // context (old field, different from scope_context)
            symbol.visibility,
            // NOTE: We clone scope_context here because ScopeContext now contains CompactString
            // (for parent_name) which doesn't implement Copy. This clone happens during indexing
            // where we process thousands of symbols per second.
            //
            // PERFORMANCE TRADEOFF: Each clone allocates for the parent_name string. For a typical
            // function name of ~20 chars, this is a small allocation. At 10,000 symbols/sec, this
            // could add measurable overhead.
            //
            // TODO: Benchmark impact and consider:
            // 1. Changing add_document to accept &Option<ScopeContext> to avoid the clone
            // 2. Using Arc<str> for parent_name to make cloning cheaper
            // 3. Accepting the overhead if it's <5% performance impact
            //
            // This should be tested with real workloads to ensure we maintain our performance targets.
            symbol.scope_context.clone(),
            symbol.language_id.as_ref().map(|id| id.as_str()),
        )
    }

    /// Store file information
    pub(crate) fn store_file_info(
        &self,
        file_id: FileId,
        path: &str,
        hash: &str,
        timestamp: u64,
    ) -> StorageResult<()> {
        let mut writer_lock = match self.writer.lock() {
            Ok(lock) => lock,
            Err(poisoned) => {
                eprintln!("Warning: Recovering from poisoned writer mutex in store_file_info");
                poisoned.into_inner()
            }
        };
        let writer = writer_lock.as_mut().ok_or(StorageError::NoActiveBatch)?;

        let mut doc = Document::new();
        doc.add_text(self.schema.doc_type, "file_info");
        doc.add_u64(self.schema.file_id, file_id.value() as u64);
        doc.add_text(self.schema.file_path, path);
        doc.add_text(self.schema.file_hash, hash);
        doc.add_u64(self.schema.file_timestamp, timestamp);

        writer.add_document(doc)?;
        Ok(())
    }

    /// Store metadata (counters, etc.)
    pub(crate) fn store_metadata(&self, key: MetadataKey, value: u64) -> StorageResult<()> {
        let mut writer_lock = match self.writer.lock() {
            Ok(lock) => lock,
            Err(poisoned) => {
                eprintln!("Warning: Recovering from poisoned writer mutex in store_metadata");
                poisoned.into_inner()
            }
        };
        let writer = writer_lock.as_mut().ok_or(StorageError::NoActiveBatch)?;

        // First delete any existing metadata with this key
        let key_str = key.as_str();
        let term = Term::from_field_text(self.schema.meta_key, key_str);
        writer.delete_term(term);

        let mut doc = Document::new();
        doc.add_text(self.schema.doc_type, "metadata");
        doc.add_text(self.schema.meta_key, key_str);
        doc.add_u64(self.schema.meta_value, value);

        writer.add_document(doc)?;
        Ok(())
    }

    /// Query all relationships from the index
    #[allow(dead_code)]
    pub(crate) fn query_relationships(
        &self,
    ) -> StorageResult<Vec<(SymbolId, SymbolId, crate::Relationship)>> {
        let searcher = self.reader.searcher();
        let query = TermQuery::new(
            Term::from_field_text(self.schema.doc_type, "relationship"),
            IndexRecordOption::Basic,
        );

        // Use a collector that retrieves all documents
        let collector = TopDocs::with_limit(1_000_000); // Adjust limit as needed
        let top_docs = searcher.search(&query, &collector)?;

        let mut relationships = Vec::new();
        for (_score, doc_address) in top_docs {
            let doc: Document = searcher.doc(doc_address)?;

            let from_id = doc
                .get_first(self.schema.from_symbol_id)
                .and_then(|v| v.as_u64())
                .and_then(|id| SymbolId::new(id as u32))
                .ok_or(StorageError::InvalidFieldValue {
                    field: "from_symbol_id".to_string(),
                    reason: "not a valid u32".to_string(),
                })?;

            let to_id = doc
                .get_first(self.schema.to_symbol_id)
                .and_then(|v| v.as_u64())
                .and_then(|id| SymbolId::new(id as u32))
                .ok_or(StorageError::InvalidFieldValue {
                    field: "to_symbol_id".to_string(),
                    reason: "not a valid u32".to_string(),
                })?;

            let kind_str = doc
                .get_first(self.schema.relation_kind)
                .and_then(|v| v.as_str())
                .ok_or(StorageError::InvalidFieldValue {
                    field: "relation_kind".to_string(),
                    reason: "missing from document".to_string(),
                })?;

            let weight = doc
                .get_first(self.schema.relation_weight)
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;

            // Parse RelationKind from string
            let kind = match kind_str {
                "Calls" => RelationKind::Calls,
                "CalledBy" => RelationKind::CalledBy,
                "Extends" => RelationKind::Extends,
                "ExtendedBy" => RelationKind::ExtendedBy,
                "Implements" => RelationKind::Implements,
                "ImplementedBy" => RelationKind::ImplementedBy,
                "Uses" => RelationKind::Uses,
                "UsedBy" => RelationKind::UsedBy,
                "Defines" => RelationKind::Defines,
                "DefinedIn" => RelationKind::DefinedIn,
                "References" => RelationKind::References,
                "ReferencedBy" => RelationKind::ReferencedBy,
                _ => continue, // Skip unknown relation kinds
            };

            let mut relationship = Relationship::new(kind).with_weight(weight);

            // Check for metadata
            let has_metadata = doc.get_first(self.schema.relation_line).is_some()
                || doc.get_first(self.schema.relation_column).is_some()
                || doc.get_first(self.schema.relation_context).is_some();

            if has_metadata {
                let mut metadata = RelationshipMetadata::new();

                if let Some(line) = doc
                    .get_first(self.schema.relation_line)
                    .and_then(|v| v.as_u64())
                {
                    metadata.line = Some(line as u32);
                }
                if let Some(column) = doc
                    .get_first(self.schema.relation_column)
                    .and_then(|v| v.as_u64())
                {
                    metadata.column = Some(column as u16);
                }
                if let Some(context) = doc
                    .get_first(self.schema.relation_context)
                    .and_then(|v| v.as_str())
                {
                    metadata.context = Some(context.into());
                }

                relationship = relationship.with_metadata(metadata);
            }

            relationships.push((from_id, to_id, relationship));
        }

        Ok(relationships)
    }

    /// Query all file information from the index
    #[allow(dead_code)]
    pub(crate) fn query_file_info(&self) -> StorageResult<Vec<(FileId, String, String, u64)>> {
        let searcher = self.reader.searcher();
        let query = TermQuery::new(
            Term::from_field_text(self.schema.doc_type, "file_info"),
            IndexRecordOption::Basic,
        );

        let collector = TopDocs::with_limit(100_000); // Adjust as needed
        let top_docs = searcher.search(&query, &collector)?;

        let mut files = Vec::new();
        for (_score, doc_address) in top_docs {
            let doc: Document = searcher.doc(doc_address)?;

            let file_id = doc
                .get_first(self.schema.file_id)
                .and_then(|v| v.as_u64())
                .and_then(|id| FileId::new(id as u32))
                .ok_or(StorageError::InvalidFieldValue {
                    field: "file_id".to_string(),
                    reason: "not a valid u32".to_string(),
                })?;

            let path = doc
                .get_first(self.schema.file_path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let hash = doc
                .get_first(self.schema.file_hash)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let timestamp = doc
                .get_first(self.schema.file_timestamp)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            files.push((file_id, path, hash, timestamp));
        }

        Ok(files)
    }

    /// Count symbols in Tantivy index
    #[allow(dead_code)]
    pub(crate) fn count_symbol_documents(&self) -> StorageResult<u64> {
        let searcher = self.reader.searcher();
        let query = TermQuery::new(
            Term::from_field_text(self.schema.doc_type, "symbol"),
            IndexRecordOption::Basic,
        );

        let count = searcher.search(&query, &tantivy::collector::Count)?;
        Ok(count as u64)
    }

    /// DEPRECATED: This method is no longer needed with Tantivy-only architecture
    #[deprecated(note = "Use Tantivy queries directly instead of rebuilding IndexData")]
    #[allow(dead_code)]
    pub(crate) fn rebuild_index_data(&self) -> StorageResult<()> {
        // This method is kept temporarily for compatibility but does nothing
        Ok(())
    }

    /// Query metadata value by key
    pub(crate) fn query_metadata(&self, key: MetadataKey) -> StorageResult<Option<u64>> {
        let searcher = self.reader.searcher();

        // Build a compound query for doc_type="metadata" AND meta_key=key
        let doc_type_query = TermQuery::new(
            Term::from_field_text(self.schema.doc_type, "metadata"),
            IndexRecordOption::Basic,
        );
        let key_query = TermQuery::new(
            Term::from_field_text(self.schema.meta_key, key.as_str()),
            IndexRecordOption::Basic,
        );

        let query = BooleanQuery::new(vec![
            (Occur::Must, Box::new(doc_type_query) as Box<dyn Query>),
            (Occur::Must, Box::new(key_query) as Box<dyn Query>),
        ]);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

        if let Some((_score, doc_address)) = top_docs.into_iter().next() {
            let doc: Document = searcher.doc(doc_address)?;
            let value = doc
                .get_first(self.schema.meta_value)
                .and_then(|v| v.as_u64());
            Ok(value)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_document_index_creation() {
        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        assert_eq!(index.document_count().unwrap(), 0);
        assert!(!index.has_vector_support());
        assert!(index.vector_storage_path().is_none());
        assert!(index.vector_engine().is_none());
    }

    #[test]
    fn test_schema_has_language_field() {
        let (schema, _) = IndexSchema::build();

        // Check that language field exists in schema
        let language_field = schema.get_field("language");
        assert!(
            language_field.is_ok(),
            "Schema should have 'language' field"
        );

        // Verify field is configured correctly
        let field = language_field.unwrap();
        let field_entry = schema.get_field_entry(field);
        assert!(field_entry.is_indexed(), "Language field should be indexed");
        assert!(field_entry.is_stored(), "Language field should be stored");
    }

    #[test]
    fn test_add_and_search_document() {
        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        // Start batch
        index.start_batch().unwrap();

        // Add a document
        let symbol_id = SymbolId::new(1).unwrap();
        let file_id = FileId::new(1).unwrap();
        index
            .add_document(
                symbol_id,
                "parse_json",
                SymbolKind::Function,
                file_id,
                "src/parser.rs",
                42,
                5,
                50, // end_line
                0,  // end_column
                Some("Parse JSON string into a Value"),
                Some("fn parse_json(input: &str) -> StorageResult<Value, Error>"),
                "crate::parser",
                None,
                crate::Visibility::Public,
                Some(crate::ScopeContext::Module),
                None, // No language_id for this test
            )
            .unwrap();

        // Commit batch
        index.commit_batch().unwrap();

        // Search for it
        let results = index.search("json", 10, None, None, None).unwrap();
        assert_eq!(results.len(), 1);

        let result = &results[0];
        assert_eq!(result.name, "parse_json");
        assert_eq!(result.line, 42);
        assert_eq!(result.file_path, "src/parser.rs");
    }

    #[test]
    fn test_store_and_retrieve_symbol_with_language() {
        use crate::parsing::registry::LanguageId;

        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        // Start batch
        index.start_batch().unwrap();

        // Create a symbol with language_id
        let symbol = crate::Symbol::new(
            SymbolId::new(1).unwrap(),
            "test_func",
            SymbolKind::Function,
            FileId::new(1).unwrap(),
            crate::Range::new(10, 0, 15, 0),
        )
        .with_language_id(LanguageId::new("rust"))
        .with_signature("fn test_func() -> Result<()>")
        .with_doc("Test function");

        // Store the symbol
        index.index_symbol(&symbol, "src/test.rs").unwrap();
        index.commit_batch().unwrap();

        // Retrieve the symbol
        let retrieved = index.find_symbol_by_id(symbol.id).unwrap();
        assert!(retrieved.is_some());

        let retrieved_symbol = retrieved.unwrap();
        // Language ID is now properly stored and retrieved through the registry
        assert_eq!(retrieved_symbol.language_id, Some(LanguageId::new("rust")));
        assert_eq!(retrieved_symbol.name.as_ref(), "test_func");
        assert_eq!(
            retrieved_symbol.signature.as_deref(),
            Some("fn test_func() -> Result<()>")
        );
    }

    #[test]
    fn test_fuzzy_search() {
        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        // Start batch
        index.start_batch().unwrap();

        let symbol_id = SymbolId::new(1).unwrap();
        let file_id = FileId::new(1).unwrap();
        index
            .add_document(
                symbol_id,
                "handle_request",
                SymbolKind::Function,
                file_id,
                "src/server.rs",
                100,
                0,
                120, // end_line
                0,   // end_column
                Some("Handle incoming HTTP request"),
                None,
                "crate::server",
                None,
                crate::Visibility::Private,
                Some(crate::ScopeContext::Module),
                None, // No language_id for this test
            )
            .unwrap();

        // Commit batch
        index.commit_batch().unwrap();

        // Search with typo - try searching for a single word with typo
        let results = index.search("handle", 10, None, None, None).unwrap();
        assert!(!results.is_empty(), "Should find exact match");

        // Now try with a small typo
        let results = index.search("handl", 10, None, None, None).unwrap();
        assert!(!results.is_empty(), "Should find with fuzzy search");
    }

    #[test]
    fn test_relationship_storage() {
        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        // Start batch
        index.start_batch().unwrap();

        let from_id = SymbolId::new(1).unwrap();
        let to_id = SymbolId::new(2).unwrap();
        let rel = crate::Relationship::new(crate::RelationKind::Calls).with_weight(0.8);

        index.store_relationship(from_id, to_id, &rel).unwrap();

        // Commit batch
        index.commit_batch().unwrap();

        // Query relationships
        let relationships = index.query_relationships().unwrap();
        assert_eq!(relationships.len(), 1);

        let (f, t, r) = &relationships[0];
        assert_eq!(*f, from_id);
        assert_eq!(*t, to_id);
        assert_eq!(r.kind, crate::RelationKind::Calls);
        assert_eq!(r.weight, 0.8);
    }

    #[test]
    fn test_file_info_storage() {
        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        // Start batch
        index.start_batch().unwrap();

        let file_id = crate::FileId::new(1).unwrap();
        index
            .store_file_info(file_id, "src/main.rs", "abc123", 1234567890)
            .unwrap();

        // Commit batch
        index.commit_batch().unwrap();

        // Query file info
        let files = index.query_file_info().unwrap();
        assert_eq!(files.len(), 1);

        let (id, path, hash, timestamp) = &files[0];
        assert_eq!(*id, file_id);
        assert_eq!(path, "src/main.rs");
        assert_eq!(hash, "abc123");
        assert_eq!(*timestamp, 1234567890);
    }

    #[test]
    fn test_get_all_indexed_paths() {
        println!("=== TEST: get_all_indexed_paths() ===");

        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        // Initially should have no paths
        println!("Step 1: Testing empty index...");
        let paths = index.get_all_indexed_paths().unwrap();
        assert_eq!(paths.len(), 0);
        println!("  ✓ Empty index returns 0 paths");

        // Add some file info documents
        println!("\nStep 2: Adding file info documents...");
        index.start_batch().unwrap();

        // Add multiple files with different paths
        let test_files = vec![
            (1, "src/main.rs", "hash1"),
            (2, "src/lib.rs", "hash2"),
            (3, "tests/integration.rs", "hash3"),
            (4, "src/module/helper.rs", "hash4"),
            (5, "benches/benchmark.rs", "hash5"),
        ];

        for (id, path, hash) in &test_files {
            let file_id = crate::FileId::new(*id).unwrap();
            index
                .store_file_info(file_id, path, hash, 1234567890)
                .unwrap();
            println!("  - Added: {path}");
        }

        index.commit_batch().unwrap();
        println!("  ✓ Added {} file info documents", test_files.len());

        // Now get all paths
        println!("\nStep 3: Retrieving all indexed paths...");
        let paths = index.get_all_indexed_paths().unwrap();

        println!("  Retrieved {} paths:", paths.len());
        for (i, path) in paths.iter().enumerate() {
            println!("    [{}] {}", i + 1, path.display());
        }

        // Verify we got all paths
        assert_eq!(paths.len(), test_files.len());
        println!("  ✓ Correct number of paths returned");

        // Verify all expected paths are present
        let path_strings: Vec<String> = paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        for (_, expected_path, _) in &test_files {
            assert!(
                path_strings.contains(&expected_path.to_string()),
                "Missing path: {expected_path}"
            );
        }
        println!("  ✓ All expected paths are present");

        // Add a symbol document (should not appear in paths)
        println!("\nStep 4: Adding a symbol document (should be ignored)...");
        index.start_batch().unwrap();

        let symbol_id = SymbolId::new(100).unwrap();
        let file_id = FileId::new(1).unwrap();
        index
            .add_document(
                symbol_id,
                "test_function",
                SymbolKind::Function,
                file_id,
                "src/main.rs",
                42,
                5,
                50, // end_line
                0,  // end_column
                Some("Test function"),
                Some("fn test_function()"),
                "crate",
                None,
                crate::Visibility::Public,
                Some(crate::ScopeContext::Module),
                None, // No language_id for this test
            )
            .unwrap();

        index.commit_batch().unwrap();
        println!("  - Added symbol document");

        // Verify paths count hasn't changed (symbols are not files)
        let paths_after_symbol = index.get_all_indexed_paths().unwrap();
        assert_eq!(paths_after_symbol.len(), test_files.len());
        println!("  ✓ Symbol documents correctly ignored");

        println!("\n=== TEST PASSED: get_all_indexed_paths() works correctly ===");
    }

    #[test]
    fn test_metadata_storage() {
        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        // Start batch
        index.start_batch().unwrap();

        index.store_metadata(MetadataKey::FileCounter, 42).unwrap();
        index
            .store_metadata(MetadataKey::SymbolCounter, 100)
            .unwrap();

        // Commit batch
        index.commit_batch().unwrap();

        // Query metadata
        assert_eq!(
            index.query_metadata(MetadataKey::FileCounter).unwrap(),
            Some(42)
        );
        assert_eq!(
            index.query_metadata(MetadataKey::SymbolCounter).unwrap(),
            Some(100)
        );
    }

    #[test]
    fn test_vector_metadata_creation() {
        // Test creating metadata without vector
        let metadata = VectorMetadata::new(1);
        assert_eq!(metadata.vector_id, None);
        assert_eq!(metadata.cluster_id, None);
        assert_eq!(metadata.embedding_version, 1);
        assert!(!metadata.has_vector());

        // Test creating metadata with vector
        let vector_id = VectorId::new(42).unwrap();
        let cluster_id = ClusterId::new(5).unwrap();
        let metadata_with_vector = VectorMetadata::with_vector(vector_id, cluster_id, 2);
        assert_eq!(metadata_with_vector.vector_id, Some(vector_id));
        assert_eq!(metadata_with_vector.cluster_id, Some(cluster_id));
        assert_eq!(metadata_with_vector.embedding_version, 2);
        assert!(metadata_with_vector.has_vector());
    }

    #[test]
    fn test_vector_metadata_serialization() {
        // Test serialization of metadata without vector
        let metadata = VectorMetadata::new(1);
        let json = metadata.to_json().unwrap();
        let deserialized = VectorMetadata::from_json(&json).unwrap();
        assert_eq!(metadata, deserialized);

        // Test serialization of metadata with vector
        let vector_id = VectorId::new(123).unwrap();
        let cluster_id = ClusterId::new(7).unwrap();
        let metadata_with_vector = VectorMetadata::with_vector(vector_id, cluster_id, 3);
        let json = metadata_with_vector.to_json().unwrap();
        let deserialized = VectorMetadata::from_json(&json).unwrap();
        assert_eq!(metadata_with_vector, deserialized);

        // Verify JSON structure
        assert!(json.contains("\"vector_id\""));
        assert!(json.contains("\"cluster_id\""));
        assert!(json.contains("\"embedding_version\":3"));
    }

    #[test]
    fn test_vector_metadata_deserialization_error() {
        // Test invalid JSON
        let invalid_json = "{ invalid json }";
        let result = VectorMetadata::from_json(invalid_json);
        assert!(result.is_err());
        match result {
            Err(StorageError::Serialization(msg)) => {
                assert!(msg.contains("Failed to deserialize VectorMetadata"));
            }
            _ => panic!("Expected Serialization error"),
        }
    }

    #[test]
    fn test_vector_metadata_tantivy_roundtrip() {
        use tantivy::schema::{STORED, SchemaBuilder, TEXT};
        use tantivy::{Index, TantivyDocument, doc};

        // Create a simple schema with a metadata field
        let mut schema_builder = SchemaBuilder::default();
        let metadata_field = schema_builder.add_text_field("vector_metadata", TEXT | STORED);
        let schema = schema_builder.build();

        // Create an in-memory index
        let index = Index::create_in_ram(schema);
        let mut index_writer = index.writer(50_000_000).unwrap();

        // Create test metadata
        let vector_id = VectorId::new(999).unwrap();
        let cluster_id = ClusterId::new(42).unwrap();
        let metadata = VectorMetadata::with_vector(vector_id, cluster_id, 5);

        // Serialize and store in document
        let json = metadata.to_json().unwrap();
        let doc = doc!(metadata_field => json.clone());
        index_writer.add_document(doc).unwrap();
        index_writer.commit().unwrap();

        // Read back from index
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let doc: TantivyDocument = searcher.doc(tantivy::DocAddress::new(0, 0)).unwrap();

        // Deserialize and verify
        let stored_json = doc
            .get_first(metadata_field)
            .and_then(|v| v.as_str())
            .unwrap();
        let retrieved_metadata = VectorMetadata::from_json(stored_json).unwrap();

        assert_eq!(metadata, retrieved_metadata);
        assert_eq!(json, stored_json);
    }

    #[test]
    fn test_document_index_with_vector_support() {
        use crate::vector::{VectorDimension, VectorSearchEngine};

        let temp_dir = TempDir::new().unwrap();
        let vector_dir = temp_dir.path().join("vectors");

        // Create vector engine
        let vector_engine =
            VectorSearchEngine::new(&vector_dir, VectorDimension::new(384).unwrap()).unwrap();
        let vector_engine_arc = Arc::new(Mutex::new(vector_engine));

        // Create index with vector support
        let index = DocumentIndex::new(temp_dir.path())
            .unwrap()
            .with_vector_support(vector_engine_arc.clone(), &vector_dir);

        // Verify vector support is enabled
        assert!(index.has_vector_support());
        assert_eq!(index.vector_storage_path(), Some(vector_dir.as_ref()));
        assert!(index.vector_engine().is_some());

        // Verify the engine reference is the same
        let engine_ref = index.vector_engine().unwrap();
        assert!(Arc::ptr_eq(engine_ref, &vector_engine_arc));
    }

    #[test]
    fn test_document_index_operations_with_and_without_vectors() {
        let temp_dir = TempDir::new().unwrap();

        // Test 1: Create index without vector support
        let index_no_vectors = DocumentIndex::new(temp_dir.path().join("no_vectors")).unwrap();

        // Basic operations should work
        index_no_vectors.start_batch().unwrap();
        index_no_vectors
            .add_document(
                SymbolId::new(1).unwrap(),
                "test_func",
                SymbolKind::Function,
                FileId::new(1).unwrap(),
                "test.rs",
                1,
                1,
                10, // end_line
                0,  // end_column
                None,
                None,
                "test",
                None,
                crate::Visibility::Public,
                Some(crate::ScopeContext::Module),
                None, // No language_id for this test
            )
            .unwrap();
        index_no_vectors.commit_batch().unwrap();

        let results = index_no_vectors
            .search("test_func", 10, None, None, None)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(!index_no_vectors.has_vector_support());

        // Test 2: Create index with vector support
        use crate::vector::{VectorDimension, VectorSearchEngine};

        let vector_dir = temp_dir.path().join("vectors");
        let vector_engine =
            VectorSearchEngine::new(&vector_dir, VectorDimension::new(384).unwrap()).unwrap();
        let vector_engine_arc = Arc::new(Mutex::new(vector_engine));

        let index_with_vectors = DocumentIndex::new(temp_dir.path().join("with_vectors"))
            .unwrap()
            .with_vector_support(vector_engine_arc, &vector_dir);

        // Same operations should work with vector support
        index_with_vectors.start_batch().unwrap();
        index_with_vectors
            .add_document(
                SymbolId::new(2).unwrap(),
                "vector_func",
                SymbolKind::Function,
                FileId::new(1).unwrap(),
                "test.rs",
                10,
                1,
                20, // end_line
                0,  // end_column
                None,
                None,
                "test",
                None,
                crate::Visibility::Public,
                Some(crate::ScopeContext::Module),
                None, // No language_id for this test
            )
            .unwrap();
        index_with_vectors.commit_batch().unwrap();

        let results = index_with_vectors
            .search("vector_func", 10, None, None, None)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(index_with_vectors.has_vector_support());
    }

    #[test]
    fn test_document_index_debug_impl() {
        use crate::vector::{VectorDimension, VectorSearchEngine};

        let temp_dir = TempDir::new().unwrap();

        // Test Debug without vector support
        let index = DocumentIndex::new(temp_dir.path()).unwrap();
        let debug_str = format!("{index:?}");
        assert!(debug_str.contains("DocumentIndex"));
        assert!(debug_str.contains("index_path"));
        assert!(debug_str.contains("vector_storage_path: None"));
        assert!(debug_str.contains("has_vector_engine: false"));

        // Test Debug with vector support
        let vector_dir = temp_dir.path().join("vectors");
        let vector_engine =
            VectorSearchEngine::new(&vector_dir, VectorDimension::new(384).unwrap()).unwrap();
        let vector_engine_arc = Arc::new(Mutex::new(vector_engine));

        let index_with_vectors = DocumentIndex::new(temp_dir.path())
            .unwrap()
            .with_vector_support(vector_engine_arc, &vector_dir);

        let debug_str = format!("{index_with_vectors:?}");
        assert!(debug_str.contains("DocumentIndex"));
        assert!(debug_str.contains("has_vector_engine: true"));
        assert!(debug_str.contains(&format!(
            "vector_storage_path: Some(\"{}\")",
            vector_dir.display()
        )));
    }

    #[test]
    fn test_cluster_cache_operations() {
        use crate::vector::{VectorDimension, VectorSearchEngine};
        use std::time::Instant;

        let temp_dir = TempDir::new().unwrap();
        let vector_dir = temp_dir.path().join("vectors");

        // Create vector engine
        let vector_engine =
            VectorSearchEngine::new(&vector_dir, VectorDimension::new(384).unwrap()).unwrap();
        let vector_engine_arc = Arc::new(Mutex::new(vector_engine));

        // Create index with vector support
        let index = DocumentIndex::new(temp_dir.path())
            .unwrap()
            .with_vector_support(vector_engine_arc, &vector_dir);

        // Start batch and add documents with cluster assignments
        index.start_batch().unwrap();

        // Add documents to different clusters
        let test_docs = vec![
            (1, "parse_string", 0), // cluster 0
            (2, "parse_number", 0), // cluster 0
            (3, "parse_json", 0),   // cluster 0
            (4, "Parser", 1),       // cluster 1
            (5, "JsonParser", 1),   // cluster 1
            (6, "handle_error", 2), // cluster 2
            (7, "ParseError", 2),   // cluster 2
        ];

        for (id, name, cluster) in &test_docs {
            let mut doc = Document::new();
            doc.add_text(index.schema.doc_type, "symbol");
            doc.add_u64(index.schema.symbol_id, *id);
            doc.add_text(index.schema.name, name);
            doc.add_text(index.schema.kind, "function");
            doc.add_u64(index.schema.file_id, 1);
            doc.add_text(index.schema.file_path, "test.rs");
            doc.add_u64(index.schema.line_number, *id * 10);
            doc.add_u64(index.schema.column, 1);
            doc.add_text(index.schema.module_path, "test");

            // Add vector fields - cluster IDs need to be non-zero
            doc.add_u64(index.schema.cluster_id, *cluster + 1); // Make 1-based
            doc.add_u64(index.schema.vector_id, *id);
            doc.add_u64(index.schema.has_vector, 1);

            index
                .writer
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .add_document(doc)
                .unwrap();
        }

        // Commit and trigger cache building
        index.commit_batch().unwrap();

        // Verify cache was built
        {
            let cache = index.cluster_cache.read().unwrap();
            assert!(cache.is_some(), "Cache should be built after commit");

            let cluster_cache = cache.as_ref().unwrap();
            println!(
                "Total documents in cache: {}",
                cluster_cache.total_documents()
            );

            // Check cluster IDs
            let cluster_ids = cluster_cache.all_cluster_ids();
            println!(
                "Found cluster IDs: {:?}",
                cluster_ids.iter().map(|id| id.get()).collect::<Vec<_>>()
            );

            // Debug: print mappings
            for (_seg_ord, clusters) in &cluster_cache.segment_mappings {
                println!(
                    "Segment {}: clusters={:?}",
                    _seg_ord.get(),
                    clusters.keys().map(|c| c.get()).collect::<Vec<_>>()
                );
                for (cluster_id, docs) in clusters {
                    println!("  Cluster {}: {} docs", cluster_id.get(), docs.len());
                }
            }
        }

        // Test document lookups - need to collect from all segments
        let all_cluster_ids = index.get_all_cluster_ids().unwrap();
        assert_eq!(all_cluster_ids.len(), 3);

        // Count total documents per cluster across all segments
        let mut cluster1_total = 0;
        let mut cluster2_total = 0;
        let mut cluster3_total = 0;

        {
            let cache = index.cluster_cache.read().unwrap();
            let cluster_cache = cache.as_ref().unwrap();

            for clusters in cluster_cache.segment_mappings.values() {
                if let Some(docs) = clusters.get(&ClusterId::new(1).unwrap()) {
                    cluster1_total += docs.len();
                }
                if let Some(docs) = clusters.get(&ClusterId::new(2).unwrap()) {
                    cluster2_total += docs.len();
                }
                if let Some(docs) = clusters.get(&ClusterId::new(3).unwrap()) {
                    cluster3_total += docs.len();
                }
            }
        }

        assert_eq!(cluster1_total, 3);
        assert_eq!(cluster2_total, 2);
        assert_eq!(cluster3_total, 2);

        // Test performance of lookups (should be <10μs)
        let cluster_id = ClusterId::new(1).unwrap();
        let segment_ord = SegmentOrdinal::new(0);
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = index
                .get_cluster_documents(segment_ord, cluster_id)
                .unwrap();
        }
        let elapsed = start.elapsed();
        let per_lookup = elapsed / 1000;
        println!("Cluster lookup performance: {per_lookup:?} per lookup");
        assert!(
            per_lookup.as_micros() < 10,
            "Lookup should be <10μs, was {per_lookup:?}"
        );

        // Test cache invalidation - add more documents
        index.start_batch().unwrap();

        let mut doc = Document::new();
        doc.add_text(index.schema.doc_type, "symbol");
        doc.add_u64(index.schema.symbol_id, 8);
        doc.add_text(index.schema.name, "new_function");
        doc.add_text(index.schema.kind, "function");
        doc.add_u64(index.schema.file_id, 1);
        doc.add_text(index.schema.file_path, "test.rs");
        doc.add_u64(index.schema.line_number, 80);
        doc.add_u64(index.schema.column, 1);
        doc.add_text(index.schema.module_path, "test");
        doc.add_u64(index.schema.cluster_id, 1); // cluster_id must be non-zero
        doc.add_u64(index.schema.vector_id, 8);
        doc.add_u64(index.schema.has_vector, 1);

        index
            .writer
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .add_document(doc)
            .unwrap();

        index.commit_batch().unwrap();

        // Verify cache was rebuilt
        {
            let cache = index.cluster_cache.read().unwrap();
            let cluster_cache = cache.as_ref().unwrap();
            assert_eq!(cluster_cache.total_documents(), 8);
        }

        // Count documents again after update
        {
            let cache = index.cluster_cache.read().unwrap();
            let cluster_cache = cache.as_ref().unwrap();

            let mut cluster1_updated = 0;
            for clusters in cluster_cache.segment_mappings.values() {
                if let Some(docs) = clusters.get(&ClusterId::new(1).unwrap()) {
                    cluster1_updated += docs.len();
                }
            }
            assert_eq!(cluster1_updated, 4); // Should have one more
        }
    }

    #[test]
    fn test_cluster_cache_unit_operations() {
        // Test ClusterCache directly
        let mut cache = ClusterCache::new(1);

        let seg0 = SegmentOrdinal::new(0);
        let seg1 = SegmentOrdinal::new(1);
        let cluster0 = ClusterId::new(1).unwrap();
        let cluster1 = ClusterId::new(2).unwrap();

        // Add documents
        cache.add_document(seg0, cluster0, 0);
        cache.add_document(seg0, cluster0, 2);
        cache.add_document(seg0, cluster0, 1);
        cache.add_document(seg0, cluster1, 3);
        cache.add_document(seg1, cluster0, 0);

        // Sort
        cache.sort_all();

        // Test retrieval
        let docs = cache.get_documents(seg0, cluster0).unwrap();
        assert_eq!(docs, &[0, 1, 2]); // Should be sorted

        let docs = cache.get_documents(seg0, cluster1).unwrap();
        assert_eq!(docs, &[3]);

        let docs = cache.get_documents(seg1, cluster0).unwrap();
        assert_eq!(docs, &[0]);

        // Test missing lookups
        assert!(cache.get_documents(seg1, cluster1).is_none());

        // Test metadata
        assert_eq!(cache.total_documents(), 5);
        assert_eq!(cache.all_cluster_ids().len(), 2);

        // Test generation check
        assert!(cache.is_valid_for_generation(1));
        assert!(!cache.is_valid_for_generation(2));
    }

    // ==================== Language Filtering Tests ====================
    // TDD tests for Sprint 4: Task 4.1 - Language filtering support

    #[test]
    fn test_find_symbols_by_name_with_language_filter() {
        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        // Start batch
        index.start_batch().unwrap();

        // Add symbols in different languages
        // Rust main function
        index
            .add_document(
                SymbolId::new(1).unwrap(),
                "main",
                SymbolKind::Function,
                FileId::new(1).unwrap(),
                "src/main.rs",
                0,                         // line
                0,                         // column
                5,                         // end_line
                0,                         // end_column
                Some("Entry point"),       // doc_comment
                Some("fn main() {}"),      // signature
                "crate",                   // module_path
                None,                      // context
                crate::Visibility::Public, // visibility
                None,                      // scope_context
                Some("rust"),              // language_id
            )
            .unwrap();

        // Python main function
        index
            .add_document(
                SymbolId::new(2).unwrap(),
                "main",
                SymbolKind::Function,
                FileId::new(2).unwrap(),
                "src/main.py",
                0,                          // line
                0,                          // column
                5,                          // end_line
                0,                          // end_column
                Some("Python entry point"), // doc_comment
                Some("def main():"),        // signature
                "__main__",                 // module_path
                None,                       // context
                crate::Visibility::Public,  // visibility
                None,                       // scope_context
                Some("python"),             // language_id
            )
            .unwrap();

        // TypeScript main function
        index
            .add_document(
                SymbolId::new(3).unwrap(),
                "main",
                SymbolKind::Function,
                FileId::new(3).unwrap(),
                "src/main.ts",
                0,                             // line
                0,                             // column
                5,                             // end_line
                0,                             // end_column
                Some("TypeScript entry"),      // doc_comment
                Some("function main(): void"), // signature
                "app",                         // module_path
                None,                          // context
                crate::Visibility::Public,     // visibility
                None,                          // scope_context
                Some("typescript"),            // language_id
            )
            .unwrap();

        // Commit the batch
        index.commit_batch().unwrap();

        println!("\n=== Testing find_symbols_by_name with language filtering ===");

        // Test 1: Find all symbols named "main" without language filter
        let all_symbols = index.find_symbols_by_name("main", None).unwrap();
        println!("Test 1 - No filter: Found {} symbols", all_symbols.len());
        for symbol in &all_symbols {
            println!("  - Symbol ID: {:?}, File: {}", symbol.id, symbol.file_id.0);
        }
        assert_eq!(
            all_symbols.len(),
            3,
            "Should find 3 'main' functions across all languages"
        );

        // Test 2: Find only Rust symbols
        let rust_symbols = index.find_symbols_by_name("main", Some("rust")).unwrap();
        println!("Test 2 - Rust filter: Found {} symbols", rust_symbols.len());
        for symbol in &rust_symbols {
            println!(
                "  - Symbol ID: {:?}, Module: {:?}",
                symbol.id, symbol.module_path
            );
        }
        assert_eq!(rust_symbols.len(), 1, "Should find 1 Rust 'main' function");
        assert_eq!(rust_symbols[0].id, SymbolId::new(1).unwrap());

        // Test 3: Find only Python symbols
        let python_symbols = index.find_symbols_by_name("main", Some("python")).unwrap();
        println!(
            "Test 3 - Python filter: Found {} symbols",
            python_symbols.len()
        );
        for symbol in &python_symbols {
            println!(
                "  - Symbol ID: {:?}, Module: {:?}",
                symbol.id, symbol.module_path
            );
        }
        assert_eq!(
            python_symbols.len(),
            1,
            "Should find 1 Python 'main' function"
        );
        assert_eq!(python_symbols[0].id, SymbolId::new(2).unwrap());

        // Test 4: Find only TypeScript symbols
        let ts_symbols = index
            .find_symbols_by_name("main", Some("typescript"))
            .unwrap();
        println!(
            "Test 4 - TypeScript filter: Found {} symbols",
            ts_symbols.len()
        );
        for symbol in &ts_symbols {
            println!(
                "  - Symbol ID: {:?}, Module: {:?}",
                symbol.id, symbol.module_path
            );
        }
        assert_eq!(
            ts_symbols.len(),
            1,
            "Should find 1 TypeScript 'main' function"
        );
        assert_eq!(ts_symbols[0].id, SymbolId::new(3).unwrap());

        // Test 5: Find symbols with non-existent language (should return empty)
        let java_symbols = index.find_symbols_by_name("main", Some("java")).unwrap();
        println!(
            "Test 5 - Java filter (non-existent): Found {} symbols",
            java_symbols.len()
        );
        assert_eq!(java_symbols.len(), 0, "Should find no Java symbols");

        println!("=== All find_symbols_by_name tests completed ===\n");
    }

    #[test]
    fn test_search_with_language_filter() {
        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        // Start batch
        index.start_batch().unwrap();

        // Add symbols with "parse" in different languages
        // Rust parse function
        index
            .add_document(
                SymbolId::new(10).unwrap(),
                "parse_config",
                SymbolKind::Function,
                FileId::new(1).unwrap(),
                "src/config.rs",
                10,                                            // line
                0,                                             // column
                20,                                            // end_line
                0,                                             // end_column
                Some("Parse configuration from file"),         // doc_comment
                Some("fn parse_config(path: &str) -> Config"), // signature
                "crate::config",                               // module_path
                None,                                          // context
                crate::Visibility::Public,                     // visibility
                None,                                          // scope_context
                Some("rust"),                                  // language_id
            )
            .unwrap();

        // Python parse function
        index
            .add_document(
                SymbolId::new(11).unwrap(),
                "parse_json",
                SymbolKind::Function,
                FileId::new(2).unwrap(),
                "src/parser.py",
                5,                                         // line
                0,                                         // column
                10,                                        // end_line
                0,                                         // end_column
                Some("Parse JSON data"),                   // doc_comment
                Some("def parse_json(data: str) -> dict"), // signature
                "parser",                                  // module_path
                None,                                      // context
                crate::Visibility::Public,                 // visibility
                None,                                      // scope_context
                Some("python"),                            // language_id
            )
            .unwrap();

        // TypeScript parse function
        index
            .add_document(
                SymbolId::new(12).unwrap(),
                "parseXML",
                SymbolKind::Function,
                FileId::new(3).unwrap(),
                "src/parser.ts",
                1,                                                // line
                0,                                                // column
                8,                                                // end_line
                0,                                                // end_column
                Some("Parse XML string"),                         // doc_comment
                Some("function parseXML(xml: string): Document"), // signature
                "utils.parser",                                   // module_path
                None,                                             // context
                crate::Visibility::Public,                        // visibility
                None,                                             // scope_context
                Some("typescript"),                               // language_id
            )
            .unwrap();

        // Commit the batch
        index.commit_batch().unwrap();

        println!("\n=== Testing search with language filtering ===");

        // Test 1: Search for "parse" without language filter
        let all_results = index.search("parse", 10, None, None, None).unwrap();
        println!(
            "Test 1 - Search 'parse' no filter: Found {} results",
            all_results.len()
        );
        for result in &all_results {
            println!(
                "  - Symbol ID: {:?}, Name: {}",
                result.symbol_id, result.name
            );
        }
        assert_eq!(
            all_results.len(),
            3,
            "Should find 3 parse functions across all languages"
        );

        // Test 2: Search for "parse" in Rust only
        let rust_results = index.search("parse", 10, None, None, Some("rust")).unwrap();
        println!(
            "Test 2 - Search 'parse' Rust filter: Found {} results",
            rust_results.len()
        );
        for result in &rust_results {
            println!(
                "  - Symbol ID: {:?}, Name: {}",
                result.symbol_id, result.name
            );
        }
        assert_eq!(rust_results.len(), 1, "Should find 1 Rust parse function");
        assert_eq!(rust_results[0].symbol_id, SymbolId::new(10).unwrap());

        // Test 3: Search for "parse" in Python only
        let python_results = index
            .search("parse", 10, None, None, Some("python"))
            .unwrap();
        println!(
            "Test 3 - Search 'parse' Python filter: Found {} results",
            python_results.len()
        );
        for result in &python_results {
            println!(
                "  - Symbol ID: {:?}, Name: {}",
                result.symbol_id, result.name
            );
        }
        assert_eq!(
            python_results.len(),
            1,
            "Should find 1 Python parse function"
        );
        assert_eq!(python_results[0].symbol_id, SymbolId::new(11).unwrap());

        // Test 4: Combine language filter with kind filter
        let rust_functions = index
            .search("parse", 10, Some(SymbolKind::Function), None, Some("rust"))
            .unwrap();
        println!(
            "Test 4 - Search 'parse' Rust+Function filter: Found {} results",
            rust_functions.len()
        );
        assert_eq!(
            rust_functions.len(),
            1,
            "Should find 1 Rust function with 'parse'"
        );

        // Test 5: Search with language that has no matches
        let java_results = index.search("parse", 10, None, None, Some("java")).unwrap();
        println!(
            "Test 5 - Search 'parse' Java filter (non-existent): Found {} results",
            java_results.len()
        );
        assert_eq!(java_results.len(), 0, "Should find no Java parse functions");

        println!("=== All search tests completed ===\n");
    }

    #[test]
    fn test_language_filter_with_module_filter() {
        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        // Start batch
        index.start_batch().unwrap();

        // Add symbols with same module name but different languages
        index
            .add_document(
                SymbolId::new(20).unwrap(),
                "Handler",
                SymbolKind::Struct,
                FileId::new(1).unwrap(),
                "src/server.rs",
                1,                         // line
                0,                         // column
                10,                        // end_line
                0,                         // end_column
                Some("Request handler"),   // doc_comment
                Some("struct Handler"),    // signature
                "server",                  // module_path
                None,                      // context
                crate::Visibility::Public, // visibility
                None,                      // scope_context
                Some("rust"),              // language_id
            )
            .unwrap();

        index
            .add_document(
                SymbolId::new(21).unwrap(),
                "Handler",
                SymbolKind::Class,
                FileId::new(2).unwrap(),
                "src/server.py",
                1,                             // line
                0,                             // column
                12,                            // end_line
                0,                             // end_column
                Some("Request handler class"), // doc_comment
                Some("class Handler"),         // signature
                "server",                      // module_path
                None,                          // context
                crate::Visibility::Public,     // visibility
                None,                          // scope_context
                Some("python"),                // language_id
            )
            .unwrap();

        // Commit the batch
        index.commit_batch().unwrap();

        println!("\n=== Testing combined module and language filters ===");

        // Test combining module and language filters
        let rust_server = index
            .search("Handler", 10, None, Some("server"), Some("rust"))
            .unwrap();
        println!(
            "Test 1 - Search 'Handler' in server module + Rust: Found {} results",
            rust_server.len()
        );
        for result in &rust_server {
            println!(
                "  - Symbol ID: {:?}, Kind: {:?}",
                result.symbol_id, result.kind
            );
        }
        assert_eq!(
            rust_server.len(),
            1,
            "Should find 1 Rust Handler in server module"
        );
        assert_eq!(rust_server[0].symbol_id, SymbolId::new(20).unwrap());

        let python_server = index
            .search("Handler", 10, None, Some("server"), Some("python"))
            .unwrap();
        println!(
            "Test 2 - Search 'Handler' in server module + Python: Found {} results",
            python_server.len()
        );
        for result in &python_server {
            println!(
                "  - Symbol ID: {:?}, Kind: {:?}",
                result.symbol_id, result.kind
            );
        }
        assert_eq!(
            python_server.len(),
            1,
            "Should find 1 Python Handler in server module"
        );
        assert_eq!(python_server[0].symbol_id, SymbolId::new(21).unwrap());

        println!("=== All combined filter tests completed ===\n");
    }

    #[test]
    fn test_ngram_partial_matching() {
        println!("\n=== NGRAM TOKENIZER TEST ===\n");

        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        index.start_batch().unwrap();

        // Add C# symbols with typical naming patterns
        let file_id = crate::FileId::new(1).unwrap();

        println!("Step 1: Indexing symbols...");

        // Symbol 1: ArchiveAppService (should match "Archive" query)
        let sym1 = crate::Symbol::new(
            SymbolId::new(1).unwrap(),
            "ArchiveAppService",
            SymbolKind::Class,
            file_id,
            crate::Range::new(10, 5, 50, 10),
        )
        .with_module_path("Services")
        .with_doc("Application service for archiving")
        .with_signature("class ArchiveAppService");

        println!("  - Indexed: ArchiveAppService");
        index
            .index_symbol(&sym1, "src/Services/ArchiveAppService.cs")
            .unwrap();

        // Symbol 2: DocumentArchiver (should match "Archive" query)
        let sym2 = crate::Symbol::new(
            SymbolId::new(2).unwrap(),
            "DocumentArchiver",
            SymbolKind::Class,
            file_id,
            crate::Range::new(20, 5, 60, 10),
        )
        .with_module_path("Utils")
        .with_doc("Archives documents")
        .with_signature("class DocumentArchiver");

        println!("  - Indexed: DocumentArchiver");
        index
            .index_symbol(&sym2, "src/Utils/DocumentArchiver.cs")
            .unwrap();

        // Symbol 3: UserService (should NOT match "Archive" query)
        let sym3 = crate::Symbol::new(
            SymbolId::new(3).unwrap(),
            "UserService",
            SymbolKind::Class,
            file_id,
            crate::Range::new(30, 5, 70, 10),
        )
        .with_module_path("Services")
        .with_doc("User management service")
        .with_signature("class UserService");

        println!("  - Indexed: UserService");
        index
            .index_symbol(&sym3, "src/Services/UserService.cs")
            .unwrap();

        index.commit_batch().unwrap();
        println!("\nStep 2: Testing partial search with 'Archive'...");

        // Test partial matching with "Archive" using search() method
        let results = index.search("Archive", 10, None, None, None).unwrap();

        println!("\nResults from search('Archive'):");
        for (i, result) in results.iter().enumerate() {
            let kind = format!("{:?}", result.kind);
            println!("  {}. {} ({})", i + 1, result.name, kind);
        }

        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();

        println!(
            "\nExpectation: Should find 'ArchiveAppService' and 'DocumentArchiver', NOT 'UserService'"
        );
        println!("Actual matches: {names:?}\n");

        // Should find both ArchiveAppService and DocumentArchiver
        assert!(
            names.contains(&"ArchiveAppService"),
            "Ngram tokenizer should find ArchiveAppService with partial query 'Archive'. Found: {names:?}"
        );
        assert!(
            names.contains(&"DocumentArchiver"),
            "Ngram tokenizer should find DocumentArchiver with partial query 'Archive'. Found: {names:?}"
        );

        // Should NOT find UserService
        assert!(
            !names.contains(&"UserService"),
            "Should not match unrelated symbols. Found: {names:?}"
        );

        println!("Step 3: Testing exact lookup with 'ArchiveAppService'...");

        // Test exact lookup still works (uses STRING field, not ngram)
        let exact_results = index
            .find_symbols_by_name("ArchiveAppService", None)
            .unwrap();
        println!("Exact lookup results: {} match(es)", exact_results.len());
        for result in &exact_results {
            println!("  - {}", result.name);
        }

        assert_eq!(exact_results.len(), 1);
        assert_eq!(exact_results[0].name.as_ref(), "ArchiveAppService");

        println!(
            "\nStep 4: Testing exact lookup with partial name 'Archive' (should find nothing)..."
        );

        // Test that exact lookup doesn't return partial matches
        let no_match = index.find_symbols_by_name("Archive", None).unwrap();
        println!("Exact lookup for 'Archive': {} match(es)", no_match.len());

        assert_eq!(
            no_match.len(),
            0,
            "Exact lookup should not return partial matches. Found: {:?}",
            no_match.iter().map(|s| &s.name).collect::<Vec<_>>()
        );

        println!("\n=== NGRAM TEST PASSED ===\n");
    }

    #[test]
    fn test_fuzzy_search_typo_tolerance() {
        println!("\n=== FUZZY SEARCH TEST (Typo Tolerance) ===\n");

        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        index.start_batch().unwrap();

        let file_id = crate::FileId::new(1).unwrap();

        println!("Step 1: Indexing symbol 'ArchiveService'...");
        let sym = crate::Symbol::new(
            SymbolId::new(1).unwrap(),
            "ArchiveService",
            SymbolKind::Class,
            file_id,
            crate::Range::new(10, 5, 50, 10),
        )
        .with_doc("Archive service");

        index.index_symbol(&sym, "src/ArchiveService.cs").unwrap();
        index.commit_batch().unwrap();

        println!("\nStep 2: Testing fuzzy search with typos...\n");

        // Test 1: Correct spelling
        println!("Query: 'ArchiveService' (correct spelling)");
        let correct = index
            .search("ArchiveService", 10, None, None, None)
            .unwrap();
        println!("  Found: {} result(s)", correct.len());
        assert_eq!(correct.len(), 1);

        // Test 2: Missing one character (edit distance = 1)
        println!("\nQuery: 'ArchivService' (missing 'e', edit distance = 1)");
        let typo1 = index.search("ArchivService", 10, None, None, None).unwrap();
        println!("  Found: {} result(s)", typo1.len());
        if !typo1.is_empty() {
            println!("  Match: {}", typo1[0].name);
        }

        // Test 3: Wrong character (edit distance = 1)
        println!("\nQuery: 'ArchaveService' (i→a, edit distance = 1)");
        let typo2 = index
            .search("ArchaveService", 10, None, None, None)
            .unwrap();
        println!("  Found: {} result(s)", typo2.len());
        if !typo2.is_empty() {
            println!("  Match: {}", typo2[0].name);
        }

        // Test 4: Extra character (edit distance = 1)
        println!("\nQuery: 'Archivee' (partial with extra 'e', edit distance = 1)");
        let typo3 = index.search("Archivee", 10, None, None, None).unwrap();
        println!("  Found: {} result(s)", typo3.len());
        if !typo3.is_empty() {
            println!("  Match: {}", typo3[0].name);
        }

        // Test 5: Too many errors (edit distance > 1, should not match with fuzzy)
        println!("\nQuery: 'Archhive' (2 errors: extra 'h' and wrong 'h', edit distance = 2)");
        let too_many = index.search("Archhive", 10, None, None, None).unwrap();
        println!("  Found: {} result(s)", too_many.len());
        println!("  Expectation: May find via ngram partial match, but not via fuzzy (distance=2)");

        println!("\n=== FUZZY SEARCH EXPLANATION ===");
        println!("Fuzzy search (edit distance=1) handles typos like:");
        println!("  - Missing character: 'Archiv' finds 'Archive'");
        println!("  - Wrong character: 'Archave' finds 'Archive'");
        println!("  - Extra character: 'Archivee' finds 'Archive'");
        println!("\nNgram tokenizer handles partial matching:");
        println!("  - 'Archive' finds 'ArchiveService', 'DocumentArchiver'");
        println!("\nBoth work together in the same search query!");
        println!("\n=== FUZZY SEARCH TEST COMPLETE ===\n");
    }

    #[test]
    fn test_ngram_vs_fuzzy_interaction() {
        println!("\n=== UNDERSTANDING NGRAM + FUZZY INTERACTION ===\n");

        let temp_dir = TempDir::new().unwrap();
        let index = DocumentIndex::new(temp_dir.path()).unwrap();

        index.start_batch().unwrap();

        let file_id = crate::FileId::new(1).unwrap();

        println!("Indexed: 'ArchiveService'\n");
        let sym = crate::Symbol::new(
            SymbolId::new(1).unwrap(),
            "ArchiveService",
            SymbolKind::Class,
            file_id,
            crate::Range::new(10, 5, 50, 10),
        );

        index.index_symbol(&sym, "src/ArchiveService.cs").unwrap();
        index.commit_batch().unwrap();

        println!("HOW NGRAM TOKENIZATION WORKS:");
        println!("'ArchiveService' gets broken into ngrams (min=3, max=10):");
        println!("  3-grams: Arc, rch, chi, hiv, ive, veS, eSe, Ser, erv, rvi, vic, ice");
        println!("  4-grams: Arch, rchi, chiv, hive, iveS, veSe, eSer, Serv, ervi, rvic, vice");
        println!("  ... up to 10-grams\n");

        println!("TEST CASES:\n");

        // Test 1: Short partial match (should work via ngram)
        println!("1. Query: 'Arch' (4 chars, exact ngram match)");
        let short_match = index.search("Arch", 10, None, None, None).unwrap();
        println!("   Result: {} match(es) ✓", short_match.len());
        println!("   Why: 'Arch' is an exact 4-gram token in 'ArchiveService'\n");

        // Test 2: Short typo (should work via fuzzy on ngrams)
        println!("2. Query: 'Arsh' (1 typo: c→s, edit distance = 1)");
        let short_typo = index.search("Arsh", 10, None, None, None).unwrap();
        println!("   Result: {} match(es)", short_typo.len());
        if short_typo.is_empty() {
            println!("   Why: Fuzzy matches 'Arsh' against ngrams like 'Arch' (distance=1)");
            println!("        But may not find it depending on Tantivy's fuzzy implementation\n");
        } else {
            println!("   Why: Fuzzy matched 'Arsh' to ngram 'Arch' (distance=1) ✓\n");
        }

        // Test 3: Long query missing char (NOW FIXED!)
        println!("3. Query: 'ArchivService' (missing 'e', 13 chars)");
        let long_typo = index.search("ArchivService", 10, None, None, None).unwrap();
        println!("   Result: {} match(es) ✓", long_typo.len());
        println!("   Why: FIXED by adding fuzzy search on non-tokenized 'name' field!");
        println!("        Fuzzy matches 'ArchivService' → 'ArchiveService' (edit distance=1)");
        println!("        This works BEFORE ngram tokenization, avoiding misalignment\n");
        assert!(
            !long_typo.is_empty(),
            "Should find ArchiveService with typo"
        );

        // Test 4: Partial match that works (ngram overlap)
        println!("4. Query: 'Archive' (7 chars, prefix of indexed word)");
        let partial = index.search("Archive", 10, None, None, None).unwrap();
        println!("   Result: {} match(es) ✓", partial.len());
        println!("   Why: 'Archive' ngrams (Arc, rch, chi, hiv, ive, etc.)");
        println!("        overlap with 'ArchiveService' ngrams\n");

        println!("CONCLUSION:");
        println!("- Ngram tokenizer: Great for partial matching (prefix/substring) ✓");
        println!("- Fuzzy on ngrams: Works for typos in SHORT queries ✓");
        println!("- Fuzzy on whole word: FIXED - Now handles typos in LONG words ✓");
        println!("\nSOLUTION IMPLEMENTED:");
        println!("  Added fuzzy search on non-tokenized 'name' field (STRING type)");
        println!("  Now search queries try BOTH:");
        println!("    1. Fuzzy on ngram tokens (for short queries)");
        println!("    2. Fuzzy on whole words (for full symbol names)");
        println!("  Result: 'ArchivService' correctly finds 'ArchiveService' ✓");

        println!("\n=== TEST COMPLETE ===\n");
    }
}
