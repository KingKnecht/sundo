# Helper Methods and Builder Patterns - Design Document

This document shows what "Create helper methods/builders for common patterns" would look like for the undo_redo library.

## Overview

Helper methods reduce boilerplate and make common operations more ergonomic. They fall into several categories:

## 1. Extension Methods for Convenience Operations

### Multi-Step Navigation
```rust
pub trait UndoRedoExt<T>: Actions<T> {
    /// Undo multiple steps at once
    fn undo_n(&mut self, n: usize) -> Option<Rc<T>>;
    
    /// Redo multiple steps at once  
    fn redo_n(&mut self, n: usize) -> Option<Rc<T>>;
    
    /// Undo all the way to the beginning
    fn undo_all(&mut self) -> Option<Rc<T>>;
    
    /// Redo all the way to the end
    fn redo_all(&mut self) -> Option<Rc<T>>;
}
```

**Benefits:**
- Avoid multiple undo/redo calls in a loop
- Common navigation patterns become one-liners
- Example: `undo_redo.undo_n(5)` instead of 5 separate undo() calls

### State Queries
```rust
pub trait UndoRedoExt<T>: Actions<T> {
    // Note: can_undo() and can_redo() already exist in the Actions trait!
    
    /// Get number of undoable steps
    fn undoable_count(&self) -> usize;
    
    /// Get number of redoable steps
    fn redoable_count(&self) -> usize;
    
    /// Peek at history entry without moving pointer
    fn peek(&self, index: usize) -> Option<Rc<T>>;
}
```

**Benefits:**
- Clearer intent than manual calculations
- UI components can easily show undo/redo button states
- Example: Enable undo button when `can_undo()` (already exists!)

## 2. Scoped Transactions (RAII Pattern) ✅ **IMPLEMENTED**

### Automatic Rollback
```rust
pub struct ScopedTransaction<'a, T> {
    undo_redo: &'a mut UndoRedo<T>,
    description: String,
    committed: bool,
}

impl<'a, T> ScopedTransaction<'a, T> {
    pub fn begin(undo_redo: &'a mut UndoRedo<T>, description: String) -> Self;
    pub fn commit(self);  // Explicit commit
    pub fn get(&mut self) -> &mut UndoRedo<T>;  // Access wrapped undo_redo
    // Drop impl: auto-rollback if not committed
}

// Also available: PersistentScopedTransaction for PersistentUndoRedo
```

**Usage:**
```rust
{
    let tx = ScopedTransaction::begin(&mut undo_redo, "Batch update".into());
    undo_redo.update(|x| x + 1, "step 1".into());
    undo_redo.update(|x| x * 2, "step 2".into());
    tx.commit();  // Success - commits transaction
}

{
    let _tx = ScopedTransaction::begin(&mut undo_redo, "This fails".into());
    undo_redo.update(|x| x + 100, "step".into());
    // Panic or early return here? Auto-rollback via Drop!
}
```

**Benefits:**
- Exception-safe transaction handling
- No need to remember to call cancel_transaction()
- Follows Rust RAII idioms (like Mutex locks)

## 3. Builder Pattern for Initialization

### Fluent Configuration
```rust
pub struct UndoRedoBuilder<T> {
    initial_capacity: usize,
    max_entries: Option<usize>,
    max_memory_bytes: Option<usize>,
}

let undo_redo = UndoRedoBuilder::new()
    .with_capacity(1000)
    .with_max_entries(500)          // Limit by count
    .with_max_memory_mb(100)        // Limit by memory (MB)
    .with_initial_state(initial_data, "Start")
    .build();
```

**Benefits:**
- Discoverable configuration options
- Self-documenting code
- Optional features without parameter explosion
- Prevent unbounded memory growth

**See also**: [MEMORY_LIMITS_DESIGN.md](MEMORY_LIMITS_DESIGN.md) for detailed design

## 4. Type-Specific Helpers

### For im::Vector
```rust
pub trait VectorOps<T: Clone> {
    /// Push item to vector
    fn vec_push(&mut self, item: T, description: impl Into<String>) -> Option<&Vector<T>>;
    
    /// Pop item from vector
    fn vec_pop(&mut self, description: impl Into<String>) -> Option<&Vector<T>>;
    
    /// Insert at index
    fn vec_insert(&mut self, index: usize, item: T, description: impl Into<String>) -> Option<&Vector<T>>;
    
    /// Remove at index
    fn vec_remove(&mut self, index: usize, description: impl Into<String>) -> Option<&Vector<T>>;
    
    /// Update at index
    fn vec_update(&mut self, index: usize, item: T, description: impl Into<String>) -> Option<&Vector<T>>;
}
```

**Instead of:**
```rust
undo_redo.update(
    |vec| {
        let mut new_vec = vec.clone();
        new_vec.push_back(42);
        new_vec
    },
    "Add 42".to_string()
);
```

**Just write:**
```rust
undo_redo.vec_push(42, "Add 42");
```

### For im::HashMap
```rust
pub trait MapOps<K: Clone, V: Clone> {
    fn map_insert(&mut self, key: K, value: V, description: impl Into<String>);
    fn map_remove(&mut self, key: &K, description: impl Into<String>);
    fn map_update<F>(&mut self, key: K, f: F, description: impl Into<String>)
    where
        F: FnOnce(Option<&V>) -> V;
}
```

**Benefits:**
- Dramatically less boilerplate
- Operations match intent directly
- Type-safe and ergonomic

## 5. Batch Operations

### Transaction Builder
```rust
pub struct BatchUpdate<'a, T> {
    undo_redo: &'a mut UndoRedo<T>,
    operations: Vec<Box<dyn FnOnce(&T) -> T>>,
}

impl<'a, T> BatchUpdate<'a, T> {
    pub fn add<F>(self, op: F) -> Self 
    where F: FnOnce(&T) -> T + 'static;
    
    pub fn apply(self) -> Option<Rc<T>>;
}
```

**Usage:**
```rust
BatchUpdate::new(&mut undo_redo, "Multiple ops")
    .add(|x| x + 1)
    .add(|x| x * 2)
    .add(|x| x - 5)
    .apply();
```

**Benefits:**
- Fluent chaining of operations
- Single transaction automatically
- Composable and reusable

##6. Macros for Common Patterns

### Field Updates
```rust
#[macro_export]
macro_rules! update_field {
    ($undo_redo:expr, $field:ident = $value:expr, $desc:expr) => {
        $undo_redo.update(
            |state| {
                let mut new_state = state.clone();
                new_state.$field = $value;
                new_state
            },
            $desc.to_string()
        )
    };
}
```

**Usage:**
```rust
update_field!(undo_redo, username = "Alice", "Set username");
update_field!(undo_redo, age = 30, "Set age");
```

### Multiple Field Updates
```rust
macro_rules! update_fields {
    ($undo_redo:expr, { $($field:ident = $value:expr),+ }, $desc:expr) => {
        $undo_redo.update(
            |state| {
                let mut new_state = state.clone();
                $(new_state.$field = $value;)+
                new_state
            },
            $desc.to_string()
        )
    };
}
```

**Usage:**
```rust
update_fields!(undo_redo, {
    name = "Alice",
    age = 30,
    city = "NYC"
}, "Update user");
```

## 7. Checkpoint/Snapshot System

### Save and Restore Points
```rust
pub struct Checkpoint<T: Clone> {
    state: Rc<T>,
    pointer: usize,
    name: String,
}

pub trait CheckpointExt<T: Clone>: Actions<T> {
    /// Create named checkpoint
    fn create_checkpoint(&self, name: String) -> Checkpoint<T>;
    
    /// Restore from checkpoint
    fn restore_checkpoint(&mut self, checkpoint: &Checkpoint<T>);
    
    /// List all checkpoints
    fn list_checkpoints(&self) -> Vec<&Checkpoint<T>>;
}
```

**Usage:**
```rust
let save_point = undo_redo.create_checkpoint("Before risky operation");

// Do some work...
undo_redo.update(|x| x + 100, "Risky");

if something_went_wrong {
    undo_redo.restore_checkpoint(&save_point);
}
```

**Benefits:**
- Long-lived save points beyond transaction scope
- Named restoration points
- Useful for "are you sure?" dialogs

## 8. History Inspection Helpers

### Rich History Queries
```rust
pub trait HistoryExt<T>: Actions<T> {
    /// Get all descriptions in history
    fn descriptions(&self) -> Vec<&str>;
    
    /// Find history entry by description
    fn find_by_description(&self, pattern: &str) -> Option<usize>;
    
    /// Get all future entries (redo-able)
    fn future_history(&self) -> Vec<HistoryEntry>;
    
    /// Get all past entries  
    fn past_history(&self) -> Vec<HistoryEntry>;
    
    /// Clear history but keep current state
    fn clear_history(&mut self);
}
```

**Benefits:**
- Build history UI components
- Search and navigate history
- Debugging and testing

## Implementation Priority

### Phase 1: Essential Helpers (High Value, Low Complexity)
1. ⚪ Extension methods: `undo_n`, `redo_n` (note: `can_undo`, `can_redo` already exist)
2. ✅ **Scoped transactions with RAII** - Implemented in src/lib.rs
3. ⚪ State query methods: `undoable_count`, `redoable_count`

### Phase 2: Type-Specific (High Value for im:: types)
4. ⚪ VectorOps trait implementation
5. ⚪ MapOps trait implementation
6. ⚪ Field update macros

### Phase 3: Advanced Features (Lower Priority)
7. ⚪ BatchUpdate builder
8. ⚪ Checkpoint system
9. ⚪ History inspection helpers
10. ⚪ Builder pattern for UndoRedo construction

**Legend**: ✅ Implemented | ⚪ Not yet implemented

## Real-World Examples

### Text Editor
```rust
// Without helpers:
editor.start_transaction();
editor.update(|doc| doc.insert_char(pos, 'a'), "temp".into());
editor.update(|doc| doc.insert_char(pos+1, 'b'), "temp".into());
editor.update(|doc| doc.insert_char(pos+2, 'c'), "temp".into());
editor.end_transaction("Type 'abc'".into());

// With helpers:
BatchUpdate::new(&mut editor, "Type 'abc'")
    .add(|doc| doc.insert_char(pos, 'a'))
    .add(|doc| doc.insert_char(pos+1, 'b'))
    .add(|doc| doc.insert_char(pos+2, 'c'))
    .apply();
```

### Todo List App
```rust
// Without type-specific helpers:
undo_redo.update(
    |todos| {
        let mut new_todos = todos.clone();
        new_todos.push_back(Todo::new("Buy milk"));
        new_todos
    },
    "Add todo".into()
);

// With helpers:
undo_redo.vec_push(Todo::new("Buy milk"), "Add todo");
```

### Form with Auto-Save
```rust
// Scoped transaction with auto-rollback
fn save_form(form: &Form, undo_redo: &mut UndoRedo<FormState>) -> Result<()> {
    let tx = ScopedTransaction::begin(undo_redo, "Update form");
    
    undo_redo.update(|s| s.with_name(&form.name), "name");
    undo_redo.update(|s| s.with_email(&form.email), "email");
    validate_email(&form.email)?;  // Might error and early return
    
    tx.commit();  // Only commits if validation passes
    Ok(())
}
```

## Summary

Helper methods provide:

✅ **Ergonomics**: Less boilerplate, more expressive code
✅ **Safety**: RAII patterns prevent forgotten cleanup
✅ **Discoverability**: IDE auto-complete shows available operations
✅ **Type Safety**: Compile-time guarantees
✅ **Composability**: Methods build on existing primitives
✅ **Performance**: Zero-cost abstractions

The library becomes easier to use while maintaining the same powerful core functionality.
