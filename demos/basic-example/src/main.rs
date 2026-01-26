use sundo::{Actions, UndoRedo, PersistentActions, PersistentUndoRedo, Persistent};
use im::{Vector, HashMap};

fn main() {
    println!("=== UndoRedo vs PersistentUndoRedo Comparison ===\n");
    
    // ============================================================================
    // Example 1: Standard UndoRedo with Vec (requires Rc wrapper)
    // ============================================================================
    println!("--- Example 1: Standard UndoRedo<Vec<i32>> ---");
    {
        let mut undo_redo = UndoRedo::new();
        
        // Push initial state
        undo_redo.push(vec![1, 2, 3], "Initial".to_string());
        
        // Present returns Option<Rc<Vec<i32>>>
        let current = undo_redo.present();
        if let Some(vec_rc) = &current {
            println!("Present: {:?}", vec_rc);
        }
        
        // Update with standard Vec - full deep copy
        undo_redo.update(
            |v| {
                let mut new_v = v.clone();
                new_v.push(4);
                new_v
            },
            "Add 4".to_string()
        );
        
        println!("After update: {:?}", undo_redo.present().unwrap());
        println!("History entries: {}\n", undo_redo.get_history_len());
    }
    
    // ============================================================================
    // Example 2: PersistentUndoRedo with im::Vector (direct storage)
    // ============================================================================
    println!("--- Example 2: PersistentUndoRedo<im::Vector<i32>> ---");
    {
        let mut undo_redo = PersistentUndoRedo::new();
        
        // Push initial state
        undo_redo.push(Vector::from(vec![1, 2, 3]), "Initial".to_string());
        
        // Update returns &im::Vector<i32> (no Rc!)
        let current = undo_redo.present();
        if let Some(vec_ref) = current {
            println!("Present: {:?}", vec_ref);
        }
        
        // Update with im::Vector - structural sharing, cheap clone
        undo_redo.update(
            |v| {
                let mut new_v = v.clone();
                new_v.push_back(4);
                new_v
            },
            "Add 4".to_string()
        );
        
        println!("After update: {:?}", undo_redo.present().unwrap());
        println!("History entries: {}\n", undo_redo.get_history_len());
    }
    
    // ============================================================================
    // Example 3: Complex state with multiple persistent structures
    // ============================================================================
    println!("--- Example 3: Complex AppState (PersistentUndoRedo) ---");
    {
        #[derive(Clone, Debug)]
        struct AppState {
            users: HashMap<u64, String>,
            messages: Vector<String>,
            counter: u32,
        }
        
        impl Persistent for AppState {}
        
        impl AppState {
            fn add_user(&self, id: u64, name: String) -> Self {
                Self {
                    users: self.users.update(id, name),
                    messages: self.messages.clone(),  // Cheap O(1) clone
                    counter: self.counter,
                }
            }
            
            fn add_message(&self, msg: String) -> Self {
                let mut new_messages = self.messages.clone();
                new_messages.push_back(msg);
                Self {
                    users: self.users.clone(),  // Cheap O(1) clone
                    messages: new_messages,
                    counter: self.counter,
                }
            }
            
            fn increment(&self) -> Self {
                Self {
                    users: self.users.clone(),
                    messages: self.messages.clone(),
                    counter: self.counter + 1,
                }
            }
        }
        
        let mut undo_redo = PersistentUndoRedo::new();
        
        // Initial state
        let initial = AppState {
            users: HashMap::new(),
            messages: Vector::new(),
            counter: 0,
        };
        undo_redo.push(initial, "Initial state".to_string());
        
        // Add users
        undo_redo.update(|s| s.add_user(1, "Alice".to_string()), "Add Alice".to_string());
        undo_redo.update(|s| s.add_user(2, "Bob".to_string()), "Add Bob".to_string());
        
        // Add messages
        undo_redo.update(|s| s.add_message("Hello!".to_string()), "Message 1".to_string());
        undo_redo.update(|s| s.add_message("World!".to_string()), "Message 2".to_string());
        
        // Increment counter
        undo_redo.update(|s| s.increment(), "Increment".to_string());
        
        let current = undo_redo.present().unwrap();
        println!("Users: {}", current.users.len());
        println!("Messages: {}", current.messages.len());
        println!("Counter: {}", current.counter);
        
        // Undo a few times
        println!("\nUndoing 3 times...");
        undo_redo.undo();
        undo_redo.undo();
        undo_redo.undo();
        
        let current = undo_redo.present().unwrap();
        println!("Users: {}", current.users.len());
        println!("Messages: {}", current.messages.len());
        println!("Counter: {}\n", current.counter);
    }
    
    // ============================================================================
    // Example 4: Transaction support (both variants)
    // ============================================================================
    println!("--- Example 4: Transaction Support ---");
    {
        let mut undo_redo = PersistentUndoRedo::new();
        undo_redo.push(Vector::from(vec![1]), "Initial".to_string());
        
        // Add one more entry before transaction
        undo_redo.update(
            |v| {
                let mut new_v = v.clone();
                new_v.push_back(10);
                new_v
            },
            "Add 10".to_string()
        );
        
        println!("Before transaction: {:?}", undo_redo.present().unwrap());
        println!("History entries: {}", undo_redo.get_history_len());
        
        println!("\nStarting transaction...");
        undo_redo.start_transaction();
        
        // Multiple updates within transaction
        for i in 2..=5 {
            undo_redo.update(
                |v| {
                    let mut new_v = v.clone();
                    new_v.push_back(i);
                    new_v
                },
                format!("Add {}", i)
            );
        }
        
        undo_redo.end_transaction("Add numbers 2-5".to_string());
        
        println!("After transaction: {:?}", undo_redo.present().unwrap());
        println!("History entries: {} (transaction replaced 'Add 10' entry!)", undo_redo.get_history_len());
        
        // Single undo goes back to initial state
        undo_redo.undo();
        println!("After undo: {:?} (back to initial state!)", undo_redo.present().unwrap());
    }
    
    println!("\n=== Key Differences ===");
    println!("1. UndoRedo<T>: Returns Rc<T>, works with any Clone type");
    println!("2. PersistentUndoRedo<T: Persistent>: Returns &T, requires Persistent trait");
    println!("3. PersistentUndoRedo eliminates Rc overhead (~16 bytes per entry)");
    println!("4. Both support transactions, undo/redo, history navigation");
    println!("5. Use PersistentUndoRedo with im::Vector, im::HashMap, etc. for best memory efficiency");
}
