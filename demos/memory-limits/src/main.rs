/// Example demonstrating memory-based history limits
/// 
/// This example shows how to use the MemoryFootprint trait to limit
/// history based on estimated memory usage rather than just entry count.

use sundo::{Actions, MemoryFootprint, UndoRedo, UndoRedoBuilder};
use im::Vector;

// Custom type that implements MemoryFootprint
#[derive(Clone)]
struct LargeDocument {
    content: Vec<String>,
    metadata: String,
}

impl MemoryFootprint for LargeDocument {
    fn memory_footprint(&self) -> usize {
        let content_size: usize = self.content.iter()
            .map(|s| s.capacity())
            .sum();
        
        std::mem::size_of::<Self>() + 
        content_size + 
        self.metadata.capacity()
    }
}

impl LargeDocument {
    fn new(lines: usize) -> Self {
        let content: Vec<String> = (0..lines)
            .map(|i| format!("This is line {} with some content to use memory", i))
            .collect();
        
        Self {
            content,
            metadata: "Document metadata".to_string(),
        }
    }
    
    fn add_line(&self, line: String) -> Self {
        let mut new_content = self.content.clone();
        new_content.push(line);
        Self {
            content: new_content,
            metadata: self.metadata.clone(),
        }
    }
    
    fn size_mb(&self) -> f64 {
        self.memory_footprint() as f64 / (1024.0 * 1024.0)
    }
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn main() {
    println!("=== Memory-Based History Limits Demo ===\n");
    
    // Example 1: Count-based limit (existing feature)
    println!("--- Example 1: Count-Based Limit ---");
    let mut count_limited = UndoRedoBuilder::new()
        .with_max_entries(5)
        .build();
    
    let doc = LargeDocument::new(100);
    count_limited.push(doc, "Initial".to_string());
    
    for i in 1..=10 {
        count_limited.update(
            |d| d.add_line(format!("Additional line {}", i)),
            format!("Add line {}", i)
        );
    }
    
    println!("Pushed 10 updates, but only {} entries kept", 
             count_limited.get_history_len());
    println!();
    
    // Example 2: Memory-based limit
    println!("--- Example 2: Memory-Based Limit ---");
    let mut memory_limited = UndoRedoBuilder::new()
        .with_max_memory_mb(1)  // 1 MB limit
        .build();
    
    let doc = LargeDocument::new(1000);
    println!("Document size: {:.2} MB", doc.size_mb());
    
    memory_limited.push(doc, "Initial".to_string());
    
    // Add several large modifications
    for i in 1..=20 {
        memory_limited.update(
            |d| {
                let mut new_doc = d.clone();
                for j in 0..100 {
                    new_doc = new_doc.add_line(format!("Batch {} line {}", i, j));
                }
                new_doc
            },
            format!("Batch add {}", i)
        );
        
        if i % 5 == 0 {
            // Manually call prune to demonstrate memory-based pruning
            memory_limited.prune();
            println!("After {} batches: {} entries in history", 
                     i, memory_limited.get_history_len());
        }
    }
    
    println!("Final history length: {}", memory_limited.get_history_len());
    println!();
    
    // Example 3: Combined limits (both count and memory)
    println!("--- Example 3: Combined Limits ---");
    let mut combined = UndoRedoBuilder::new()
        .with_max_entries(100)    // Max 100 entries
        .with_max_memory_mb(2)    // Max 2 MB
        .build();
    
    let doc = LargeDocument::new(500);
    combined.push(doc, "Initial".to_string());
    
    for i in 1..=150 {
        combined.update(
            |d| d.add_line(format!("Line {}", i)),
            format!("Add line {}", i)
        );
        
        if i % 50 == 0 {
            combined.prune();
            println!("After {} updates: {} entries kept", 
                     i, combined.get_history_len());
        }
    }
    
    println!();
    
    // Example 4: Memory tracking with im::Vector
    println!("--- Example 4: Memory Footprint with im::Vector ---");
    let mut vec_history = UndoRedoBuilder::new()
        .with_max_memory_mb(1)
        .build();
    
    let vec = Vector::<i32>::new();
    vec_history.push(vec, "Empty vector".to_string());
    
    for i in 1..=1000 {
        vec_history.update(
            |v| {
                let mut new_v = v.clone();
                new_v.push_back(i);
                new_v
            },
            format!("Add {}", i)
        );
        
        if i % 200 == 0 {
            vec_history.prune();
            if let Some(current) = vec_history.present() {
                let footprint = current.memory_footprint();
                println!("After {} adds: {} entries, estimated footprint: {}", 
                         i, 
                         vec_history.get_history_len(),
                         format_bytes(footprint));
            }
        }
    }
    
    println!();
    
    // Example 5: Export snapshot with memory info
    println!("--- Example 5: Snapshot with Memory Info ---");
    let snapshot = memory_limited.export_snapshot();
    println!("Snapshot captured:");
    println!("  Entries: {}", snapshot.entries.len());
    println!("  Pointer: {}", snapshot.pointer);
    println!("  Max entries: {:?}", snapshot.max_entries);
    println!("  Max memory: {:?}", snapshot.max_memory_bytes.map(format_bytes));
    println!("  Current memory: {}", format_bytes(snapshot.current_memory_bytes));
    
    // Reconstruct from snapshot
    let restored: UndoRedo<LargeDocument> = UndoRedo::from_snapshot(snapshot);
    println!("Restored from snapshot: {} entries", restored.get_history_len());
    
    println!("\n=== Demo Complete ===");
}
