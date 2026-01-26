/// Example demonstrating the builder pattern with history limits
/// 
/// This shows how to use UndoRedoBuilder to configure history size limits,
/// which is useful for long-running applications or memory-constrained environments.

use sundo::{Actions, PersistentActions, PersistentUndoRedoBuilder, UndoRedoBuilder};
use im::Vector;

fn main() {
    println!("=== Builder Pattern with History Limits ===\n");
    
    // Example 1: Default (unlimited history)
    println!("1. Default unlimited history:");
    {
        let mut undo_redo = UndoRedoBuilder::new().build();
        
        for i in 0..100 {
            undo_redo.push(i, format!("Value {}", i));
        }
        
        println!("   Added 100 entries");
        println!("   History length: {}", undo_redo.get_history_len());
        println!("   Current value: {}", *undo_redo.present().unwrap());
    }
    
    // Example 2: Limited history (count-based)
    println!("\n2. Count-based history limit:");
    {
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_entries(10)
            .build();
        
        for i in 0..100 {
            undo_redo.push(i, format!("Value {}", i));
        }
        
        println!("   Added 100 entries");
        println!("   History length: {} (limited to 10)", undo_redo.get_history_len());
        println!("   Current value: {}", *undo_redo.present().unwrap());
        println!("   Oldest kept entry was: {}", *undo_redo.undo().unwrap());
        undo_redo.redo(); // Go back to present
    }
    
    // Example 3: Text editor simulation
    println!("\n3. Text editor with limited undo history:");
    {
        let mut editor = UndoRedoBuilder::new()
            .with_capacity(1000)  // Pre-allocate for performance
            .with_max_entries(50)  // Keep last 50 edits
            .build();
        
        editor.push("Hello".to_string(), "Initial".to_string());
        
        // Simulate typing
        let words = vec!["world", "this", "is", "a", "test", "of", "the", "undo", "system"];
        for word in words {
            editor.update(|text| format!("{} {}", text, word), format!("Add '{}'", word));
        }
        
        println!("   Document: '{}'", editor.present().unwrap());
        println!("   Can undo {} steps", editor.get_pointer());
        
        // Undo a few changes
        editor.undo();
        editor.undo();
        println!("   After 2 undos: '{}'", editor.present().unwrap());
    }
    
    // Example 4: Application with large state
    println!("\n4. Large state with aggressive pruning:");
    {
        #[derive(Clone)]
        struct AppState {
            data: Vec<u8>,
            version: usize,
        }
        
        let mut app_undo = UndoRedoBuilder::new()
            .with_max_entries(5)  // Only keep 5 states
            .build();
        
        // Initial state with 1KB of data
        app_undo.push(
            AppState {
                data: vec![0; 1024],
                version: 0,
            },
            "Initial".to_string(),
        );
        
        // Make 20 updates
        for i in 1..=20 {
            app_undo.update(
                |state| AppState {
                    data: state.data.clone(),
                    version: i,
                },
                format!("Version {}", i),
            );
        }
        
        println!("   Made 20 updates");
        println!("   History length: {} (limited to 5)", app_undo.get_history_len());
        println!("   Current version: {}", app_undo.present().unwrap().version);
        
        // Can only undo to the oldest kept entry
        let mut undo_count = 0;
        while app_undo.can_undo() {
            app_undo.undo();
            undo_count += 1;
        }
        println!("   Could undo {} times", undo_count);
        println!("   Oldest version available: {}", app_undo.present().unwrap().version);
    }
    
    // Example 5: Persistent data structures with limits
    println!("\n5. Persistent data structures (im::Vector):");
    {
        let mut undo_redo = PersistentUndoRedoBuilder::new()
            .with_max_entries(10)
            .build();
        
        undo_redo.push(Vector::new(), "Empty".to_string());
        
        // Build up a vector
        for i in 0..50 {
            undo_redo.update(|v| {
                let mut new_v = v.clone();
                new_v.push_back(i);
                new_v
            }, format!("Add {}", i));
        }
        
        println!("   Added 50 items to vector");
        println!("   History length: {} (limited to 10)", undo_redo.get_history_len());
        println!("   Vector length: {}", undo_redo.present().unwrap().len());
        
        // Undo a few times
        undo_redo.undo();
        undo_redo.undo();
        undo_redo.undo();
        println!("   After 3 undos, vector length: {}", undo_redo.present().unwrap().len());
    }
    
    // Example 6: Transaction with limits
    println!("\n6. Transactions with history limits:");
    {
        use sundo::ScopedTransaction;
        
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_entries(5)
            .build();
        
        undo_redo.push(0, "Initial".to_string());
        
        // Make several transactions
        for batch in 0..10 {
            let mut tx = ScopedTransaction::begin(&mut undo_redo, format!("Batch {}", batch));
            for i in 0..3 {
                tx.get().update(|x| x + 1, format!("temp {}", i));
            }
            tx.commit();
        }
        
        println!("   Made 10 transactions (30 total updates)");
        println!("   History length: {} (limited to 5)", undo_redo.get_history_len());
        println!("   Final value: {}", *undo_redo.present().unwrap());
    }
    
    println!("\n=== Benefits of History Limits ===");
    println!("✓ Prevents unbounded memory growth");
    println!("✓ Predictable memory usage for long-running apps");
    println!("✓ Configurable per use case");
    println!("✓ Zero cost when not configured (default unlimited)");
    println!("✓ Automatic pruning - no manual management needed");
    
    println!("\n=== Use Cases ===");
    println!("• Text editors: Limit to last N edits");
    println!("• Graphics apps: Limit undo stack to prevent OOM");
    println!("• Mobile apps: Conservative limits for memory constraints");
    println!("• Real-time systems: Bounded undo for deterministic behavior");
    println!("• Multi-document apps: Each document gets limited history");
}
