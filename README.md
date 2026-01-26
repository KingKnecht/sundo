# Sundo - Snapshot Undo/Redo

A flexible and efficient undo/redo library for Rust, with first-class support for persistent data structures.

## Features

- **Generic undo/redo** for any cloneable type
- **Optimized for persistent data structures** (`im::Vector`, `im::HashMap`, etc.)
- **Transaction support** with automatic rollback (RAII)
- **History limits** (count-based and memory-based) to prevent unbounded memory growth
- **In-place updates** with `replace_current()` for ephemeral state (navigation, UI state)
- **Serialization support** via snapshot export/import
- **Builder pattern** for flexible configuration
- **Type-safe** and ergonomic API
- **Zero-cost abstractions** when features aren't used

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
sundo = "0.1.0"
```

### Basic Usage

```rust
use sundo::{Actions, UndoRedo};

let mut undo_redo = UndoRedo::new();

// Push initial state
undo_redo.push(42, "Initial".to_string());

// Make changes
undo_redo.update(|x| x + 10, "Add 10".to_string());
undo_redo.update(|x| x * 2, "Multiply by 2".to_string());

// Undo and redo
assert_eq!(*undo_redo.present().unwrap(), 104);
undo_redo.undo();
assert_eq!(*undo_redo.present().unwrap(), 52);
undo_redo.redo();
assert_eq!(*undo_redo.present().unwrap(), 104);
```

### With Persistent Data Structures

```rust
use sundo::{PersistentActions, PersistentUndoRedo};
use im::Vector;

let mut undo_redo = PersistentUndoRedo::new();

undo_redo.push(Vector::new(), "Empty".to_string());
undo_redo.update(|v| v.push_back(1), "Add 1".to_string());
undo_redo.update(|v| v.push_back(2), "Add 2".to_string());

// No Rc overhead - direct storage with structural sharing
let current = undo_redo.present().unwrap();
assert_eq!(current.len(), 2);
```

### With History Limits

```rust
use sundo::UndoRedoBuilder;

// Count-based limit
let mut undo_redo = UndoRedoBuilder::new()
    .with_max_entries(100)  // Keep only last 100 states
    .build();

// Memory-based limit
let mut undo_redo = UndoRedoBuilder::new()
    .with_max_memory_mb(50)  // Keep up to 50 MB of history
    .build();

// Combined limits (whichever is hit first)
let mut undo_redo = UndoRedoBuilder::new()
    .with_max_entries(1000)
    .with_max_memory_mb(100)
    .build();

// Automatically prunes oldest entries when limit exceeded
```

### With Transactions

```rust
use sundo::{Actions, UndoRedo, ScopedTransaction};

let mut undo_redo = UndoRedo::new();
undo_redo.push(0, "Initial".to_string());

{
    let mut tx = ScopedTransaction::begin(&mut undo_redo, "Batch update");
    tx.get().update(|x| x + 1, "temp".to_string());
    tx.get().update(|x| x + 2, "temp".to_string());
    tx.commit();  // Commits as single entry "Batch update"
}

// Or auto-rollback on error:
{
    let mut tx = ScopedTransaction::begin(&mut undo_redo, "Risky operation");
    tx.get().update(|x| x + 100, "temp".to_string());
    // If we panic or return early, transaction auto-rolls back
}
```

### Ephemeral State Updates

For state changes that shouldn't be undoable (like navigation or UI state):

```rust
use sundo::{PersistentActions, PersistentUndoRedo};
use im::{HashMap, Vector};

#[derive(Clone)]
struct AppState {
    todos: Vector<String>,
    current_page: String,  // Navigation - shouldn't be undoable
}

let mut undo_redo = PersistentUndoRedo::new();
// ... add some todos (undoable) ...

// Change page without creating history entry
undo_redo.replace_current(|state| AppState {
    todos: state.todos.clone(),
    current_page: "Settings".to_string(),
});

// Undo still works on todos, navigation stays at Settings
undo_redo.undo();  // Reverts todo changes, keeps current_page
```

## API Overview

### Core Traits

- **`Actions<T>`** - Standard undo/redo operations with `Rc` wrapping
- **`PersistentActions<T>`** - Optimized for persistent data structures (no `Rc`)

### Core Types

- **`UndoRedo<T>`** - Standard implementation for any cloneable type
- **`PersistentUndoRedo<T>`** - Optimized for `im::` types

### Builder Pattern

- **`UndoRedoBuilder<T>`** - Configure with capacity and limits
- **`PersistentUndoRedoBuilder<T>`** - Builder for persistent variant

### Transactions

- **`ScopedTransaction`** - RAII transaction with auto-rollback
- **`PersistentScopedTransaction`** - Transaction for persistent types

### Memory Estimation

- **`MemoryFootprint`** - Trait for memory-based history limits

## Examples

See the `demos/` directory for complete examples:

- **basic-example** - Comparison of standard vs persistent implementations
- **persistent-example** - Deep dive into persistent data structures  
- **scoped-transaction** - Transaction patterns and error handling with RAII
- **builder-pattern** - History limits configuration
- **memory-limits** - Memory-based limits with MemoryFootprint trait
- **mem-performance** - Comprehensive memory performance testing and analysis
- **serialization-example** - Save/load history without serde
- **delta-example** - Compute diffs between history states

Run demos with:
```bash
cd demos/basic-example && cargo run
cd demos/memory-limits && cargo run
cd demos/delta-example && cargo run
# etc.
```

## Design Philosophy

### Memory Efficiency

For standard types, sundo uses `Rc<T>` to minimize memory overhead when cloning is expensive. For persistent data structures (which already use structural sharing), sundo stores values directly without the `Rc` layer.

### Persistent Data Structures

When using `im::Vector`, `im::HashMap`, etc., the library leverages their built-in structural sharing. This means history entries share most of their data, making undo/redo memory-efficient even for large collections.

### Zero-Cost Abstractions

- Builder pattern only allocates when configured
- History limits only check when set
- Transaction overhead is minimal (single Option field)

## Documentation

- [Usage Guide](docs/USAGE_GUIDE.md) - Comprehensive guide for choosing between UndoRedo and PersistentUndoRedo

Generate API documentation locally:
```bash
cargo doc --open
```

## Use Cases

- we will find out...

## License

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
