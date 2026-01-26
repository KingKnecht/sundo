// Example: Using sundo with persistent data structures

use sundo::{Actions, UndoRedo, Persistent};
use im::{Vector, HashMap};

// A state using persistent data structures
#[derive(Clone)]
struct AppState {
    users: HashMap<u64, String>,
    messages: Vector<String>,
    counter: u32,
}

// Mark as Persistent since it's cheap to clone (uses structural sharing)
impl Persistent for AppState {}

impl AppState {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
            messages: Vector::new(),
            counter: 0,
        }
    }
    
    // Helper methods that leverage structural sharing
    fn add_user(&self, id: u64, name: String) -> Self {
        Self {
            users: self.users.update(id, name),
            messages: self.messages.clone(),  // Cheap! Just ref bump
            counter: self.counter,
        }
    }
    
    fn add_message(&self, msg: String) -> Self {
        let mut new_messages = self.messages.clone();
        new_messages.push_back(msg);
        Self {
            users: self.users.clone(),        // Cheap!
            messages: new_messages,
            counter: self.counter,
        }
    }
    
    fn increment(&self) -> Self {
        Self {
            users: self.users.clone(),        // Cheap!
            messages: self.messages.clone(),  // Cheap!
            counter: self.counter + 1,
        }
    }
}

fn main() {
    println!("=== Persistent Data Structures Example ===\n");
    
    let mut undo_redo = UndoRedo::<AppState>::new();
    
    // Initial state
    let state = AppState::new();
    undo_redo.push(state, "Initial state".to_string());
    
    // Add users - only users HashMap changes, messages are shared
    undo_redo.update(
        |s| s.add_user(1, "Alice".to_string()),
        "Add Alice".to_string()
    );
    
    undo_redo.update(
        |s| s.add_user(2, "Bob".to_string()),
        "Add Bob".to_string()
    );
    
    // Add messages - only messages Vector changes, users are shared
    undo_redo.update(
        |s| s.add_message("Hello from Alice".to_string()),
        "Add message".to_string()
    );
    
    undo_redo.update(
        |s| s.add_message("Hello from Bob".to_string()),
        "Add message".to_string()
    );
    
    // Increment counter
    undo_redo.update(
        |s| s.increment(),
        "Increment counter".to_string()
    );
    
    // Display history
    if let Some(history) = undo_redo.get_history() {
        println!("History:");
        for entry in history {
            println!("  [{}] {}: {:?}", entry.pointer, entry.description, entry.tense);
        }
    }
    
    println!("\nCurrent state:");
    if let Some(current) = undo_redo.present() {
        println!("  Users: {}", current.users.len());
        println!("  Messages: {}", current.messages.len());
        println!("  Counter: {}", current.counter);
    }
    
    // Undo a few times
    println!("\n--- Undoing 3 times ---");
    undo_redo.undo();
    undo_redo.undo();
    undo_redo.undo();
    
    if let Some(current) = undo_redo.present() {
        println!("After undo:");
        println!("  Users: {}", current.users.len());
        println!("  Messages: {}", current.messages.len());
        println!("  Counter: {}", current.counter);
    }
    
    // Test transactions with persistent data
    println!("\n--- Testing transaction ---");
    undo_redo.start_transaction();
    
    undo_redo.update(|s| s.add_message("Transaction msg 1".to_string()), "Temp 1".to_string());
    undo_redo.update(|s| s.add_message("Transaction msg 2".to_string()), "Temp 2".to_string());
    undo_redo.update(|s| s.increment(), "Temp 3".to_string());
    
    undo_redo.end_transaction("Batch update".to_string());
    
    println!("After transaction:");
    if let Some(current) = undo_redo.present() {
        println!("  Users: {}", current.users.len());
        println!("  Messages: {}", current.messages.len());
        println!("  Counter: {}", current.counter);
    }
    
    // Display final history
    if let Some(history) = undo_redo.get_history() {
        println!("\nFinal history:");
        for entry in history {
            println!("  [{}] {}: {:?}", entry.pointer, entry.description, entry.tense);
        }
    }
    
    println!("\n💡 Key benefit: Even though we have 6+ history entries,");
    println!("   most of the data is shared between versions thanks to");
    println!("   structural sharing in im::Vector and im::HashMap!");
}
