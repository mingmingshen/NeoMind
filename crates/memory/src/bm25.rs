//! BM25 full-text search implementation.
//!
//! BM25 (Best Matching 25) is a ranking function used by search engines
//! to estimate the relevance of documents to a given search query.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Default BM25 parameters.
pub const DEFAULT_K1: f64 = 1.2;
pub const DEFAULT_B: f64 = 0.75;

/// BM25 document statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStats {
    /// Document ID
    pub id: String,
    /// Document length (number of terms)
    pub length: usize,
    /// Term frequencies in this document
    pub term_freqs: HashMap<String, usize>,
}

impl DocumentStats {
    /// Create a new document stats from text.
    pub fn new(id: impl Into<String>, text: &str) -> Self {
        let terms = tokenize(text);
        let mut term_freqs = HashMap::new();

        for term in &terms {
            *term_freqs.entry(term.clone()).or_insert(0) += 1;
        }

        Self {
            id: id.into(),
            length: terms.len(),
            term_freqs,
        }
    }

    /// Get term frequency for a given term.
    pub fn tf(&self, term: &str) -> usize {
        self.term_freqs.get(term).copied().unwrap_or(0)
    }
}

/// BM25 index for full-text search.
pub struct BM25Index {
    /// All document statistics
    docs: Vec<DocumentStats>,
    /// Document frequency: number of documents containing each term
    doc_freqs: HashMap<String, usize>,
    /// Total number of documents
    num_docs: usize,
    /// Average document length
    avg_doc_length: f64,
    /// BM25 parameter k1 (controls term frequency saturation)
    k1: f64,
    /// BM25 parameter b (controls document length normalization)
    b: f64,
}

impl BM25Index {
    /// Create a new BM25 index.
    pub fn new() -> Self {
        Self::with_params(DEFAULT_K1, DEFAULT_B)
    }

    /// Create a new BM25 index with custom parameters.
    pub fn with_params(k1: f64, b: f64) -> Self {
        Self {
            docs: Vec::new(),
            doc_freqs: HashMap::new(),
            num_docs: 0,
            avg_doc_length: 0.0,
            k1,
            b,
        }
    }

    /// Add a document to the index.
    pub fn add_document(&mut self, id: impl Into<String>, text: &str) {
        let stats = DocumentStats::new(id, text);

        // Update document frequencies
        let unique_terms: HashSet<_> = stats.term_freqs.keys().cloned().collect();
        for term in unique_terms {
            *self.doc_freqs.entry(term).or_insert(0) += 1;
        }

        self.docs.push(stats);
        self.num_docs += 1;
        self.recalculate_avg_length();
    }

    /// Add multiple documents to the index.
    pub fn add_documents<'a, I>(&mut self, documents: I)
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        for (id, text) in documents {
            self.add_document(id, text);
        }
    }

    /// Remove a document from the index.
    pub fn remove_document(&mut self, id: &str) {
        if let Some(pos) = self.docs.iter().position(|d| d.id == id) {
            let doc = &self.docs[pos];

            // Update document frequencies
            let unique_terms: HashSet<_> = doc.term_freqs.keys().cloned().collect();
            for term in unique_terms {
                if let Some(df) = self.doc_freqs.get_mut(&term) {
                    *df = df.saturating_sub(1);
                    if *df == 0 {
                        self.doc_freqs.remove(&term);
                    }
                }
            }

            self.docs.remove(pos);
            self.num_docs = self.docs.len();
            self.recalculate_avg_length();
        }
    }

    /// Search for documents matching the query.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<BM25Result> {
        let query_terms = tokenize(query);
        let mut scores: Vec<(String, f64)> = Vec::new();

        for doc in &self.docs {
            let score = self.score_document(doc, &query_terms);
            if score > 0.0 {
                scores.push((doc.id.clone(), score));
            }
        }

        // Sort by score (descending)
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Keep top_k results
        scores
            .into_iter()
            .take(top_k)
            .map(|(id, score)| BM25Result { id, score })
            .collect()
    }

    /// Calculate BM25 score for a document.
    fn score_document(&self, doc: &DocumentStats, query_terms: &[String]) -> f64 {
        if self.num_docs == 0 || self.avg_doc_length == 0.0 {
            return 0.0;
        }

        let mut score = 0.0;

        for term in query_terms {
            let tf = doc.tf(term) as f64;

            if tf == 0.0 {
                continue;
            }

            let df = *self.doc_freqs.get(term).unwrap_or(&1) as f64;
            let idf = idf(self.num_docs, df);
            let doc_length = doc.length as f64;

            // BM25 formula
            let numerator = tf * (self.k1 + 1.0);
            let denominator = tf + self.k1 * (1.0 - self.b + self.b * (doc_length / self.avg_doc_length));

            score += idf * (numerator / denominator);
        }

        score
    }

    /// Get the number of documents in the index.
    pub fn len(&self) -> usize {
        self.num_docs
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.num_docs == 0
    }

    /// Clear all documents from the index.
    pub fn clear(&mut self) {
        self.docs.clear();
        self.doc_freqs.clear();
        self.num_docs = 0;
        self.avg_doc_length = 0.0;
    }

    /// Recalculate average document length.
    fn recalculate_avg_length(&mut self) {
        if self.docs.is_empty() {
            self.avg_doc_length = 0.0;
        } else {
            let total: usize = self.docs.iter().map(|d| d.length).sum();
            self.avg_doc_length = total as f64 / self.docs.len() as f64;
        }
    }

    /// Get document statistics by ID.
    pub fn get_document(&self, id: &str) -> Option<&DocumentStats> {
        self.docs.iter().find(|d| d.id == id)
    }
}

impl Default for BM25Index {
    fn default() -> Self {
        Self::new()
    }
}

/// BM25 search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BM25Result {
    /// Document ID
    pub id: String,
    /// BM25 relevance score
    pub score: f64,
}

/// Calculate IDF (Inverse Document Frequency).
fn idf(num_docs: usize, doc_freq: f64) -> f64 {
    let n = num_docs as f64;
    if doc_freq == 0.0 {
        return 0.0;
    }

    ((n - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln()
}

/// Tokenize text into terms.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Extract text from a conversation entry for BM25 indexing.
pub fn extract_text_for_bm25(user_input: &str, assistant_response: &str) -> String {
    format!("{} {}", user_input, assistant_response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_index_creation() {
        let index = BM25Index::new();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_bm25_add_document() {
        let mut index = BM25Index::new();
        index.add_document("doc1", "hello world");
        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
    }

    #[test]
    fn test_bm25_search() {
        let mut index = BM25Index::new();
        index.add_document("doc1", "hello world");
        index.add_document("doc2", "hello rust");
        index.add_document("doc3", "goodbye world");

        let results = index.search("hello", 10);
        assert!(!results.is_empty());

        // "hello" appears in doc1 and doc2
        let ids: Vec<_> = results.iter().map(|r| &r.id).collect();
        assert!(ids.contains(&&"doc1".to_string()) || ids.contains(&&"doc2".to_string()));
    }

    #[test]
    fn test_bm25_relevance_ordering() {
        let mut index = BM25Index::new();
        index.add_document("doc1", "hello hello hello");
        index.add_document("doc2", "hello");

        let results = index.search("hello", 10);
        assert_eq!(results.len(), 2);

        // doc1 should rank higher due to higher term frequency
        assert_eq!(results[0].id, "doc1");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_bm25_remove_document() {
        let mut index = BM25Index::new();
        index.add_document("doc1", "hello world");
        index.add_document("doc2", "hello rust");

        assert_eq!(index.len(), 2);

        index.remove_document("doc1");
        assert_eq!(index.len(), 1);

        let results = index.search("hello", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc2");
    }

    #[test]
    fn test_bm25_clear() {
        let mut index = BM25Index::new();
        index.add_document("doc1", "hello world");
        index.add_document("doc2", "hello rust");

        assert_eq!(index.len(), 2);

        index.clear();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());

        let results = index.search("hello", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_bm25_custom_params() {
        let index = BM25Index::with_params(1.5, 0.5);
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_document_stats() {
        let stats = DocumentStats::new("doc1", "hello world hello");
        assert_eq!(stats.id, "doc1");
        assert_eq!(stats.length, 3);
        assert_eq!(stats.tf("hello"), 2);
        assert_eq!(stats.tf("world"), 1);
        assert_eq!(stats.tf("unknown"), 0);
    }

    #[test]
    fn test_tokenize() {
        let terms = tokenize("Hello, World! This is a Test.");
        assert_eq!(terms.len(), 6);
        assert!(terms.contains(&"hello".to_string()));
        assert!(terms.contains(&"world".to_string()));
        assert!(terms.contains(&"this".to_string()));
    }

    #[test]
    fn test_extract_text_for_bm25() {
        let text = extract_text_for_bm25("What is AI?", "AI stands for Artificial Intelligence");
        assert!(text.contains("AI"));
        assert!(text.contains("Artificial"));
    }
}
