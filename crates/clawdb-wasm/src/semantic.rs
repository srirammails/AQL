//! Pre-seeded Semantic Memory with vector similarity
//!
//! Contains ~20 tech concepts with pre-computed embeddings.
//! Read-only - users can query but not add new records.

use serde::{Deserialize, Serialize};

/// A semantic record with pre-computed embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRecord {
    pub id: String,
    pub concept: String,
    pub knowledge: String,
    pub category: String,
    #[serde(skip_serializing)]
    pub embedding: Vec<f32>,
}

/// Pre-seeded semantic store
pub struct SemanticStore {
    records: Vec<SemanticRecord>,
}

impl SemanticStore {
    pub fn new() -> Self {
        Self {
            records: create_preseeded_data(),
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Search by similarity to query text
    /// Uses simple keyword matching + pre-computed similarity scores
    pub fn search(&self, query: &str, limit: usize) -> Vec<SemanticResult> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f32, &SemanticRecord)> = self.records.iter()
            .map(|record| {
                let score = compute_similarity(record, &query_words);
                (score, record)
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter()
            .take(limit)
            .map(|(score, record)| SemanticResult {
                id: record.id.clone(),
                concept: record.concept.clone(),
                knowledge: record.knowledge.clone(),
                category: record.category.clone(),
                similarity: score,
            })
            .collect()
    }
}

/// Result from semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticResult {
    pub id: String,
    pub concept: String,
    pub knowledge: String,
    pub category: String,
    pub similarity: f32,
}

/// Compute similarity score based on keyword matching
/// Returns score between 0.0 and 1.0
fn compute_similarity(record: &SemanticRecord, query_words: &[&str]) -> f32 {
    let concept_lower = record.concept.to_lowercase();
    let knowledge_lower = record.knowledge.to_lowercase();
    let category_lower = record.category.to_lowercase();

    let mut score = 0.0;
    let mut matches = 0;

    for word in query_words {
        // Exact concept match is highest
        if concept_lower == *word {
            score += 1.0;
            matches += 1;
        }
        // Concept contains word
        else if concept_lower.contains(word) {
            score += 0.8;
            matches += 1;
        }
        // Category match
        else if category_lower.contains(word) {
            score += 0.6;
            matches += 1;
        }
        // Knowledge contains word
        else if knowledge_lower.contains(word) {
            score += 0.4;
            matches += 1;
        }
    }

    if matches > 0 {
        score / query_words.len() as f32
    } else {
        0.0
    }
}

/// Create pre-seeded semantic data
fn create_preseeded_data() -> Vec<SemanticRecord> {
    vec![
        // Programming Languages
        SemanticRecord {
            id: "sem-001".to_string(),
            concept: "rust".to_string(),
            knowledge: "Systems programming language focused on safety, concurrency, and performance. Memory safe without garbage collection.".to_string(),
            category: "programming".to_string(),
            embedding: vec![0.12, -0.34, 0.56], // Placeholder
        },
        SemanticRecord {
            id: "sem-002".to_string(),
            concept: "python".to_string(),
            knowledge: "High-level dynamic programming language emphasizing readability. Popular for data science, AI, and scripting.".to_string(),
            category: "programming".to_string(),
            embedding: vec![0.15, -0.31, 0.52],
        },
        SemanticRecord {
            id: "sem-003".to_string(),
            concept: "typescript".to_string(),
            knowledge: "Typed superset of JavaScript that compiles to plain JavaScript. Adds static typing to JavaScript.".to_string(),
            category: "programming".to_string(),
            embedding: vec![0.18, -0.28, 0.48],
        },
        SemanticRecord {
            id: "sem-004".to_string(),
            concept: "golang".to_string(),
            knowledge: "Statically typed compiled language by Google. Known for simplicity, concurrency with goroutines, and fast compilation.".to_string(),
            category: "programming".to_string(),
            embedding: vec![0.14, -0.32, 0.54],
        },

        // Infrastructure
        SemanticRecord {
            id: "sem-005".to_string(),
            concept: "kubernetes".to_string(),
            knowledge: "Container orchestration platform for automating deployment, scaling, and management of containerized applications.".to_string(),
            category: "infrastructure".to_string(),
            embedding: vec![0.45, 0.22, -0.18],
        },
        SemanticRecord {
            id: "sem-006".to_string(),
            concept: "docker".to_string(),
            knowledge: "Container runtime platform for building, shipping, and running applications in isolated environments.".to_string(),
            category: "infrastructure".to_string(),
            embedding: vec![0.42, 0.25, -0.15],
        },
        SemanticRecord {
            id: "sem-007".to_string(),
            concept: "terraform".to_string(),
            knowledge: "Infrastructure as Code tool for building, changing, and versioning infrastructure safely and efficiently.".to_string(),
            category: "infrastructure".to_string(),
            embedding: vec![0.38, 0.28, -0.12],
        },

        // Databases
        SemanticRecord {
            id: "sem-008".to_string(),
            concept: "postgresql".to_string(),
            knowledge: "Advanced open-source relational database with strong SQL compliance, ACID transactions, and extensibility.".to_string(),
            category: "database".to_string(),
            embedding: vec![-0.22, 0.55, 0.33],
        },
        SemanticRecord {
            id: "sem-009".to_string(),
            concept: "redis".to_string(),
            knowledge: "In-memory data structure store used as database, cache, and message broker. Supports strings, hashes, lists, sets.".to_string(),
            category: "database".to_string(),
            embedding: vec![-0.18, 0.52, 0.36],
        },
        SemanticRecord {
            id: "sem-010".to_string(),
            concept: "mongodb".to_string(),
            knowledge: "Document-oriented NoSQL database storing data in flexible JSON-like documents with dynamic schemas.".to_string(),
            category: "database".to_string(),
            embedding: vec![-0.20, 0.48, 0.38],
        },
        SemanticRecord {
            id: "sem-011".to_string(),
            concept: "lancedb".to_string(),
            knowledge: "Embedded vector database for AI applications. Stores embeddings alongside metadata for similarity search.".to_string(),
            category: "database".to_string(),
            embedding: vec![-0.15, 0.58, 0.30],
        },

        // AI/ML
        SemanticRecord {
            id: "sem-012".to_string(),
            concept: "transformer".to_string(),
            knowledge: "Neural network architecture using self-attention mechanisms. Foundation of modern LLMs like GPT and BERT.".to_string(),
            category: "ai".to_string(),
            embedding: vec![0.65, -0.12, 0.44],
        },
        SemanticRecord {
            id: "sem-013".to_string(),
            concept: "embedding".to_string(),
            knowledge: "Dense vector representation of data (text, images) in continuous space where similar items are close together.".to_string(),
            category: "ai".to_string(),
            embedding: vec![0.62, -0.15, 0.48],
        },
        SemanticRecord {
            id: "sem-014".to_string(),
            concept: "rag".to_string(),
            knowledge: "Retrieval Augmented Generation - combining LLMs with external knowledge retrieval for accurate responses.".to_string(),
            category: "ai".to_string(),
            embedding: vec![0.58, -0.18, 0.52],
        },
        SemanticRecord {
            id: "sem-015".to_string(),
            concept: "llm".to_string(),
            knowledge: "Large Language Model - neural networks trained on vast text data capable of understanding and generating human language.".to_string(),
            category: "ai".to_string(),
            embedding: vec![0.68, -0.10, 0.42],
        },

        // Web/APIs
        SemanticRecord {
            id: "sem-016".to_string(),
            concept: "graphql".to_string(),
            knowledge: "Query language for APIs allowing clients to request exactly the data they need. Alternative to REST.".to_string(),
            category: "api".to_string(),
            embedding: vec![0.28, 0.42, 0.18],
        },
        SemanticRecord {
            id: "sem-017".to_string(),
            concept: "rest".to_string(),
            knowledge: "Representational State Transfer - architectural style for distributed systems using HTTP methods and resources.".to_string(),
            category: "api".to_string(),
            embedding: vec![0.25, 0.45, 0.15],
        },
        SemanticRecord {
            id: "sem-018".to_string(),
            concept: "websocket".to_string(),
            knowledge: "Protocol for full-duplex communication over TCP. Enables real-time bidirectional data transfer.".to_string(),
            category: "api".to_string(),
            embedding: vec![0.22, 0.48, 0.12],
        },

        // Security
        SemanticRecord {
            id: "sem-019".to_string(),
            concept: "oauth".to_string(),
            knowledge: "Open authorization standard for token-based authentication. Allows third-party access without sharing passwords.".to_string(),
            category: "security".to_string(),
            embedding: vec![-0.35, -0.28, 0.62],
        },
        SemanticRecord {
            id: "sem-020".to_string(),
            concept: "jwt".to_string(),
            knowledge: "JSON Web Token - compact URL-safe means of representing claims for authentication and information exchange.".to_string(),
            category: "security".to_string(),
            embedding: vec![-0.32, -0.25, 0.58],
        },
    ]
}

impl Default for SemanticStore {
    fn default() -> Self {
        Self::new()
    }
}
