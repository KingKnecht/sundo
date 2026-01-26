/// Example demonstrating the ScopedTransaction RAII pattern
/// 
/// This shows how to use automatic rollback for exception-safe transaction handling.

use sundo::{Actions, ScopedTransaction, UndoRedo};

fn main() {
    println!("=== ScopedTransaction Demo ===\n");
    
    let mut undo_redo = UndoRedo::new();
    undo_redo.push(10, "Initial".to_string());
    
    println!("Initial value: {}", *undo_redo.present().unwrap());
    println!("History length: {}\n", undo_redo.get_history_len());
    
    // Example 1: Transaction that commits successfully
    println!("1. Committed transaction:");
    {
        let mut tx = ScopedTransaction::begin(&mut undo_redo, "Add 30");
        tx.get().update(|x| x + 10, "temp".to_string());
        tx.get().update(|x| x + 10, "temp".to_string());
        tx.get().update(|x| x + 10, "temp".to_string());
        tx.commit();  // Explicit commit
    }
    println!("   After commit: {}", *undo_redo.present().unwrap());
    println!("   History length: {}", undo_redo.get_history_len());
    println!("   (3 updates collapsed into 1 transaction)\n");
    
    // Example 2: Transaction that rolls back automatically
    println!("2. Auto-rollback transaction:");
    let before_rollback = *undo_redo.present().unwrap();
    {
        let mut tx = ScopedTransaction::begin(&mut undo_redo, "This will rollback");
        tx.get().update(|x| x + 1000, "temp".to_string());
        tx.get().update(|x| x * 2, "temp".to_string());
        println!("   Inside transaction: {}", *tx.get().present().unwrap());
        // tx drops here without commit -> automatic rollback!
    }
    println!("   After auto-rollback: {}", *undo_redo.present().unwrap());
    println!("   Value unchanged: {}\n", *undo_redo.present().unwrap() == before_rollback);
    
    // Example 3: Exception-safe error handling
    println!("3. Exception-safe with early return:");
    fn risky_operation(undo_redo: &mut UndoRedo<i32>) -> Result<(), &'static str> {
        let mut tx = ScopedTransaction::begin(undo_redo, "Risky operation");
        
        tx.get().update(|x| x + 100, "temp".to_string());
        tx.get().update(|x| x + 100, "temp".to_string());
        
        // Simulate an error condition
        if *tx.get().present().unwrap() > 200 {
            return Err("Value too large!");  // Early return -> auto-rollback
        }
        
        tx.get().update(|x| x + 100, "temp".to_string());
        tx.commit();  // This line never reached in our case
        Ok(())
    }
    
    let before_risky = *undo_redo.present().unwrap();
    match risky_operation(&mut undo_redo) {
        Ok(_) => println!("   Operation succeeded"),
        Err(e) => println!("   Operation failed: {}", e),
    }
    println!("   Value after failure: {}", *undo_redo.present().unwrap());
    println!("   Rolled back: {}\n", *undo_redo.present().unwrap() == before_risky);
    
    // Example 4: Nested scope safety
    println!("4. Scope isolation:");
    {
        let mut tx1 = ScopedTransaction::begin(&mut undo_redo, "First transaction");
        tx1.get().update(|x| x + 5, "temp".to_string());
        tx1.commit();
    }
    println!("   After first transaction: {}", *undo_redo.present().unwrap());
    
    {
        let mut tx2 = ScopedTransaction::begin(&mut undo_redo, "Second transaction");
        tx2.get().update(|x| x * 2, "temp".to_string());
        // Not committing this one
    }
    println!("   After second (rolled back): {}", *undo_redo.present().unwrap());
    println!("   History entries: {}", undo_redo.get_history_len());
    
    // Show final history
    println!("\n=== Final History ===");
    if let Some(history) = undo_redo.get_history() {
        for entry in history {
            println!("   {:?} [{}] - {}", 
                entry.tense, 
                entry.pointer, 
                entry.description
            );
        }
    }
    
    println!("\n=== Benefits ===");
    println!("✓ Automatic rollback on panic or early return");
    println!("✓ No need to manually call cancel_transaction()");
    println!("✓ Exception-safe by design (RAII pattern)");
    println!("✓ Works like Mutex/MutexGuard - familiar Rust idiom");
}
