/// Delta Computation Example
/// 
/// This demo shows how to compute deltas (diffs) between states in undo/redo history.
/// Since payloads are generic, you implement diffing for your specific types.

use sundo::{Actions, UndoRedo};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
struct Document {
    title: String,
    content: String,
    metadata: HashMap<String, String>,
}

impl Document {
    fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            content: String::new(),
            metadata: HashMap::new(),
        }
    }
    
    fn set_content(&self, content: &str) -> Self {
        Self {
            title: self.title.clone(),
            content: content.to_string(),
            metadata: self.metadata.clone(),
        }
    }
    
    fn set_metadata(&self, key: &str, value: &str) -> Self {
        let mut metadata = self.metadata.clone();
        metadata.insert(key.to_string(), value.to_string());
        Self {
            title: self.title.clone(),
            content: self.content.clone(),
            metadata,
        }
    }
}

// Delta types for different aspects of the document
#[derive(Debug, PartialEq)]
enum DocumentDelta {
    TitleChanged { from: String, to: String },
    ContentChanged { from: String, to: String },
    MetadataAdded { key: String, value: String },
    MetadataRemoved { key: String },
    MetadataChanged { key: String, from: String, to: String },
    Multiple(Vec<DocumentDelta>),
}

// Trait for types that support delta computation
trait Diffable {
    type Delta;
    
    fn diff(&self, other: &Self) -> Option<Self::Delta>;
}

impl Diffable for Document {
    type Delta = DocumentDelta;
    
    fn diff(&self, other: &Self) -> Option<Self::Delta> {
        let mut deltas = Vec::new();
        
        // Check title
        if self.title != other.title {
            deltas.push(DocumentDelta::TitleChanged {
                from: self.title.clone(),
                to: other.title.clone(),
            });
        }
        
        // Check content
        if self.content != other.content {
            deltas.push(DocumentDelta::ContentChanged {
                from: self.content.clone(),
                to: other.content.clone(),
            });
        }
        
        // Check metadata
        for (key, old_value) in &self.metadata {
            match other.metadata.get(key) {
                Some(new_value) if new_value != old_value => {
                    deltas.push(DocumentDelta::MetadataChanged {
                        key: key.clone(),
                        from: old_value.clone(),
                        to: new_value.clone(),
                    });
                }
                None => {
                    deltas.push(DocumentDelta::MetadataRemoved { key: key.clone() });
                }
                _ => {}
            }
        }
        
        for (key, value) in &other.metadata {
            if !self.metadata.contains_key(key) {
                deltas.push(DocumentDelta::MetadataAdded {
                    key: key.clone(),
                    value: value.clone(),
                });
            }
        }
        
        match deltas.len() {
            0 => None,
            1 => Some(deltas.into_iter().next().unwrap()),
            _ => Some(DocumentDelta::Multiple(deltas)),
        }
    }
}

impl DocumentDelta {
    fn describe(&self) -> String {
        match self {
            DocumentDelta::TitleChanged { from, to } => {
                format!("Title: '{}' → '{}'", from, to)
            }
            DocumentDelta::ContentChanged { from, to } => {
                let from_preview = if from.len() > 20 {
                    format!("{}...", &from[..20])
                } else {
                    from.clone()
                };
                let to_preview = if to.len() > 20 {
                    format!("{}...", &to[..20])
                } else {
                    to.clone()
                };
                format!("Content: '{}' → '{}'", from_preview, to_preview)
            }
            DocumentDelta::MetadataAdded { key, value } => {
                format!("Metadata added: {} = '{}'", key, value)
            }
            DocumentDelta::MetadataRemoved { key } => {
                format!("Metadata removed: {}", key)
            }
            DocumentDelta::MetadataChanged { key, from, to } => {
                format!("Metadata changed: {} = '{}' → '{}'", key, from, to)
            }
            DocumentDelta::Multiple(deltas) => {
                format!("{} changes: {}", deltas.len(), 
                    deltas.iter()
                        .map(|d| d.describe())
                        .collect::<Vec<_>>()
                        .join(", "))
            }
        }
    }
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║               Delta Computation Example                        ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    
    // Create document with history
    let mut undo_redo = UndoRedo::<Document>::new();
    
    let doc = Document::new("My Document");
    undo_redo.push(doc, "Created document".to_string());
    
    undo_redo.update(|d| d.set_content("Hello"), "Added greeting".to_string());
    undo_redo.update(|d| d.set_content("Hello World"), "Expanded greeting".to_string());
    undo_redo.update(|d| d.set_metadata("author", "Alice"), "Set author".to_string());
    undo_redo.update(|d| d.set_metadata("version", "1.0"), "Set version".to_string());
    
    println!("Document History:");
    if let Some(history) = undo_redo.get_history() {
        for (i, entry) in history.iter().enumerate() {
            let marker = if entry.pointer == undo_redo.get_pointer() {
                "→"
            } else {
                " "
            };
            println!("  {} [{}] {}", marker, i, entry.description);
        }
    }
    println!();
    
    // Compute deltas between consecutive states
    println!("Computing Deltas Between Consecutive States:");
    println!();
    
    for i in 1..undo_redo.get_history_len() {
        let prev = undo_redo.peek_at(i - 1).unwrap();
        let curr = undo_redo.peek_at(i).unwrap();
        
        if let Some(delta) = prev.diff(&*curr) {
            println!("  Step {} → {}: {}", i - 1, i, delta.describe());
        }
    }
    println!();
    
    // Compare current state with beginning
    println!("Delta from Beginning to Current State:");
    let initial = undo_redo.peek_at(0).unwrap();
    let current = undo_redo.present().unwrap();
    
    if let Some(delta) = initial.diff(&*current) {
        println!("  {}", delta.describe());
    }
    println!();
    
    // Compare any two arbitrary states
    println!("Comparing Arbitrary States:");
    let state_1 = undo_redo.peek_at(1).unwrap();
    let state_3 = undo_redo.peek_at(3).unwrap();
    
    if let Some(delta) = state_1.diff(&*state_3) {
        println!("  From state 1 to state 3: {}", delta.describe());
    }
    println!();
    
    // Show use case: Generating changelog
    println!("Use Case: Generating Changelog:");
    println!();
    println!("  Document Changelog:");
    for i in 1..undo_redo.get_history_len() {
        let prev = undo_redo.peek_at(i - 1).unwrap();
        let curr = undo_redo.peek_at(i).unwrap();
        
        if let Some(delta) = prev.diff(&*curr) {
            let history = undo_redo.get_history().unwrap();
            println!("  • {} - {}", history[i].description, delta.describe());
        }
    }
    println!();
    
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║ Key Takeaways:                                                 ║");
    println!("║ • Use peek_at() to access states without moving pointer        ║");
    println!("║ • Implement Diffable trait for your types                      ║");
    println!("║ • Compare any two states in history                            ║");
    println!("║ • Great for: changelogs, auditing, conflict resolution         ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
}
