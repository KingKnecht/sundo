/// Serialization Example!
/// 
/// This demo shows how to serialize/deserialize undo/redo history
/// using the built-in snapshot API. You can use any serialization
/// format you prefer (JSON, bincode, custom format, etc.)

use sundo::{Actions, UndoRedo, Snapshot};

#[derive(Clone, Debug, PartialEq)]
struct Document {
    title: String,
    content: String,
    version: u32,
}

impl Document {
    fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            content: String::new(),
            version: 1,
        }
    }
    
    fn set_content(&self, content: &str) -> Self {
        Self {
            title: self.title.clone(),
            content: content.to_string(),
            version: self.version + 1,
        }
    }
}

// Simple custom serialization (you could use serde, bincode, JSON, etc.)
fn serialize_document(doc: &Document) -> String {
    format!("{}|{}|{}", doc.title, doc.content, doc.version)
}

fn deserialize_document(s: &str) -> Document {
    let parts: Vec<&str> = s.split('|').collect();
    Document {
        title: parts[0].to_string(),
        content: parts[1].to_string(),
        version: parts[2].parse().unwrap(),
    }
}

fn serialize_snapshot(snapshot: &Snapshot<Document>) -> Vec<String> {
    let mut lines = vec![
        format!("pointer:{}", snapshot.pointer),
        format!("transaction_start:{:?}", snapshot.transaction_start),
        format!("max_entries:{:?}", snapshot.max_entries),
        format!("entries:{}", snapshot.entries.len()),
    ];
    
    for (doc, desc) in &snapshot.entries {
        lines.push(format!("entry:{}::{}", serialize_document(doc), desc));
    }
    
    lines
}

fn deserialize_snapshot(lines: &[String]) -> Snapshot<Document> {
    use std::rc::Rc;
    
    let pointer = lines[0].strip_prefix("pointer:").unwrap().parse().unwrap();
    let transaction_start = if lines[1].contains("Some") {
        let num = lines[1].strip_prefix("transaction_start:Some(").unwrap()
            .strip_suffix(")").unwrap().parse().unwrap();
        Some(num)
    } else {
        None
    };
    let max_entries = if lines[2].contains("Some") {
        let num = lines[2].strip_prefix("max_entries:Some(").unwrap()
            .strip_suffix(")").unwrap().parse().unwrap();
        Some(num)
    } else {
        None
    };
    
    let mut entries = Vec::new();
    for line in &lines[4..] {
        if let Some(rest) = line.strip_prefix("entry:") {
            let parts: Vec<&str> = rest.split("::").collect();
            let doc = deserialize_document(parts[0]);
            let desc = parts[1].to_string();
            entries.push((Rc::new(doc), desc));
        }
    }
    
    Snapshot {
        entries,
        pointer,
        transaction_start,
        max_entries,
        // this demo does not use memory-based limits
        max_memory_bytes: None,
        current_memory_bytes: 0,
    }
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║         Serialization Example!                                 ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    
    // Create and modify a document with undo/redo
    let mut undo_redo = UndoRedo::<Document>::new();
    
    let doc = Document::new("My Document");
    undo_redo.push(doc, "Created document".to_string());
    
    undo_redo.update(|d| d.set_content("Hello"), "Added greeting".to_string());
    undo_redo.update(|d| d.set_content("Hello world"), "Completed sentence".to_string());
    undo_redo.update(|d| d.set_content("Hello world!"), "Added punctuation".to_string());
    
    println!("Original UndoRedo state:");
    println!("  History length: {}", undo_redo.get_history_len());
    println!("  Current pointer: {}", undo_redo.get_pointer());
    println!("  Current content: {:?}", undo_redo.present().unwrap().content);
    println!();
    
    // Export to snapshot
    println!("Exporting to snapshot...");
    let snapshot = undo_redo.export_snapshot();
    println!("  Exported {} entries", snapshot.entries.len());
    println!();
    
    // Serialize to custom format (could be JSON, bincode, etc.)
    println!("Serializing to custom text format...");
    let serialized = serialize_snapshot(&snapshot);
    println!("  Serialized to {} lines:", serialized.len());
    for (i, line) in serialized.iter().take(5).enumerate() {
        println!("    {}: {}", i, line);
    }
    if serialized.len() > 5 {
        println!("    ... ({} more lines)", serialized.len() - 5);
    }
    println!();
    
    // Simulate storage (in real app: write to file, database, network, etc.)
    println!("Simulating storage...");
    let stored_data = serialized;
    println!("  Stored {} bytes", stored_data.join("\n").len());
    println!();
    
    // Deserialize
    println!("Loading from storage...");
    let loaded_snapshot = deserialize_snapshot(&stored_data);
    println!("  Loaded {} entries", loaded_snapshot.entries.len());
    println!();
    
    // Reconstruct UndoRedo from snapshot
    println!("Reconstructing UndoRedo...");
    let mut restored = UndoRedo::from_snapshot(loaded_snapshot);
    println!("  Restored history length: {}", restored.get_history_len());
    println!("  Restored pointer: {}", restored.get_pointer());
    println!("  Restored content: {:?}", restored.present().unwrap().content);
    println!();
    
    // Verify it works
    println!("Verification:");
    println!("  Can undo: {}", restored.can_undo());
    println!("  Can redo: {}", restored.can_redo());
    
    restored.undo();
    println!("  After undo: {:?}", restored.present().unwrap().content);
    
    restored.redo();
    println!("  After redo: {:?}", restored.present().unwrap().content);
    println!();
    
    // Test with new modifications
    println!("Making new changes to restored state...");
    restored.update(|d| d.set_content("Hello world! (restored)"), "Modified after restore".to_string());
    println!("  New content: {:?}", restored.present().unwrap().content);
    println!("  New history length: {}", restored.get_history_len());
    
        println!();
        println!("╔════════════════════════════════════════════════════════════════╗");
        println!("║ Success! UndoRedo fully serialized and restored                ║"); 
        println!("╚════════════════════════════════════════════════════════════════╝");
    }
    
    