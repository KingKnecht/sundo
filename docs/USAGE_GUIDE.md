# Usage Guide: UndoRedo vs PersistentUndoRedo

## Quick Decision Guide

### Use `UndoRedo<T>` when:

- Working with simple types like `String`, `i32`, or small structs
- Payload type doesn't support structural sharing
- Using standard library collections (`Vec`, `HashMap`, `HashSet`)
- Prototyping or when memory isn't a primary concern
- Need to work with any `Clone` type without additional trait requirements

```rust
use undo_redo::sundo::{Actions, UndoRedo};

let mut undo_redo = UndoRedo::new();
undo_redo.push(vec![1, 2, 3], "Initial".to_string());

// Returns Rc<Vec<i32>>
let current = undo_redo.present().unwrap();
```

### Use `PersistentUndoRedo<T>` when:

- Working with `im` crate types (`im::Vector`, `im::HashMap`, etc.)
- Building complex application state with many undo operations
- Memory efficiency is important (eliminates ~16 bytes + heap allocation per entry)
- Payload has internal structural sharing
- Want to avoid double indirection (`Rc` + internal sharing)

```rust
use undo_redo::sundo::{PersistentActions, PersistentUndoRedo, Persistent};
use im::Vector;

let mut undo_redo = PersistentUndoRedo::new();
undo_redo.push(Vector::from(vec![1, 2, 3]), "Initial".to_string());

// Returns &Vector<i32> (no Rc!)
let current = undo_redo.present().unwrap();
```

## API Differences

| Feature | UndoRedo<T> | PersistentUndoRedo<T> |
|---------|-------------|----------------------|
| Trait requirement | `Clone` | `Clone + Persistent` |
| Return type | `Option<Rc<T>>` | `Option<&T>` |
| Storage | `Vec<Entry<Rc<T>>>` | `Vec<PersistentEntry<T>>` |
| Memory overhead | Rc wrapper (16 bytes) | No wrapper |
| Heap allocations | One per entry for Rc | Zero extra |
| Clone cost | Depends on T | Expected to be O(1) or O(log n) |

## Custom Types with PersistentUndoRedo

For custom types, implement `Persistent` marker trait:

```rust
use im::{HashMap, Vector};
use undo_redo::sundo::Persistent;

#[derive(Clone)]
struct AppState {
    users: HashMap<u64, String>,
    messages: Vector<String>,
    counter: u32,
}

// Mark as persistent - ensures structural sharing for im types
impl Persistent for AppState {}

impl AppState {
    fn add_user(&self, id: u64, name: String) -> Self {
        Self {
            users: self.users.update(id, name),  // O(log n)
            messages: self.messages.clone(),      // O(1)
            counter: self.counter,
        }
    }
}
```

## Memory Savings Example

From benchmarks with 11 history entries:

- `UndoRedo<Vec<i32>>`: 550 MB (full clones)
- `UndoRedo<im::Vector<i32>>`: ~77 MB (Rc + structural sharing)
- `PersistentUndoRedo<im::Vector<i32>>`: ~61 MB (structural sharing only)

**Savings: 88.9% vs standard Vec, ~20% vs UndoRedo with im::Vector**

## Both Support

- Transactions (collapse multiple operations)
- Undo/Redo navigation
- History inspection
- Go to specific history point
- Custom descriptions for each change
- Memory and count-based history limits

## History Limits

Both variants support optional history limits to prevent unbounded memory growth:

### Count-Based Limits

```rust
use undo_redo::sundo::{Actions, UndoRedoBuilder};

// Limit to 100 most recent entries
let mut undo_redo = UndoRedoBuilder::new()
    .with_max_entries(100)
    .build();
```

### Memory-Based Limits

For types implementing `MemoryFootprint`, you can limit by estimated memory usage:

```rust
use undo_redo::sundo::{Actions, MemoryFootprint, UndoRedoBuilder};
use im::Vector;

// Limit to 10 MB of history
let mut undo_redo = UndoRedoBuilder::new()
    .with_max_memory_mb(10)
    .build();

// Custom types can implement MemoryFootprint:
#[derive(Clone)]
struct MyState {
    data: Vector<String>,
    counter: u32,
}

impl MemoryFootprint for MyState {
    fn memory_footprint(&self) -> usize {
        std::mem::size_of::<Self>() +
        self.data.memory_footprint()
    }
}
```

### Combined Limits

You can use both limits together - whichever is hit first triggers pruning:

```rust
let mut undo_redo = UndoRedoBuilder::new()
    .with_max_entries(1000)    // Max 1000 entries
    .with_max_memory_mb(50)    // Max 50 MB
    .build();
```

Built-in `MemoryFootprint` implementations exist for:
- All `im::` types (`Vector`, `HashMap`, `OrdMap`, `HashSet`, `OrdSet`)
- `String` and `Vec<T>`
- Primitive numeric types (`i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`, `char`, etc.)

## Advanced Features

### Scoped Transactions (RAII Pattern)

Transactions allow you to group multiple operations into a single undo/redo entry. Scoped transactions use RAII to automatically rollback if not explicitly committed:

```rust
use undo_redo::sundo::{Actions, UndoRedo, ScopedTransaction};

let mut undo_redo = UndoRedo::new();
undo_redo.push(0, "Initial".to_string());

// Transaction that commits
{
    let mut tx = ScopedTransaction::begin(&mut undo_redo, "Batch update");
    tx.get().update(|x| x + 1, "temp".to_string());
    tx.get().update(|x| x + 2, "temp".to_string());
    tx.get().update(|x| x + 3, "temp".to_string());
    tx.commit();  // All 3 operations collapsed into "Batch update"
}

// Transaction that auto-rolls back (exception-safe)
{
    let mut tx = ScopedTransaction::begin(&mut undo_redo, "Risky operation");
    tx.get().update(|x| x + 100, "temp".to_string());
    // If panic or early return here, changes are automatically rolled back!
    // Forgot to commit? Auto-rollback via Drop
}
```

For persistent types, use `PersistentScopedTransaction`:

```rust
use im::Vector;
use undo_redo::sundo::{PersistentActions, PersistentUndoRedo, PersistentScopedTransaction};

let mut undo_redo = PersistentUndoRedo::new();
undo_redo.push(Vector::new(), "Initial".to_string());

{
    let mut tx = PersistentScopedTransaction::begin(&mut undo_redo, "Add items");
    tx.get().update(|v| {
        let mut new_v = v.clone();
        new_v.push_back(1);
        new_v
    }, "temp".to_string());
    tx.get().update(|v| {
        let mut new_v = v.clone();
        new_v.push_back(2);
        new_v
    }, "temp".to_string());
    tx.commit();
}
```

**Benefits:**
- Exception-safe: automatic rollback on panic or early return
- Less boilerplate than manual `start_transaction()`/`end_transaction()`
- Compiler ensures you don't forget to commit or cancel
- Follows Rust RAII idioms (like `MutexGuard`)

### Ephemeral State Updates with `replace_current()`

In Elmish-style applications, some state changes shouldn't be undoable (like navigation, UI state, cursor position). The `replace_current()` method modifies the current state in-place without creating a new history entry:

```rust
use im::{HashMap, Vector};
use undo_redo::sundo::{PersistentActions, PersistentUndoRedo, Persistent};

#[derive(Clone)]
struct AppState {
    todos: Vector<String>,           // Undoable data
    current_page: String,             // Ephemeral navigation
    selected_index: Option<usize>,   // Ephemeral UI state
}

impl Persistent for AppState {}

let mut undo_redo = PersistentUndoRedo::new();
let initial = AppState {
    todos: Vector::new(),
    current_page: "Home".to_string(),
    selected_index: None,
};
undo_redo.push(initial, "Initial".to_string());

// Add a todo (creates history entry)
undo_redo.update(|state| AppState {
    todos: state.todos.push_back("Buy milk".to_string()),
    ..state.clone()
}, "Add todo".to_string());

// Navigate to settings (no history entry)
undo_redo.replace_current(|state| AppState {
    current_page: "Settings".to_string(),
    ..state.clone()
});

// Undo reverts todo change but keeps navigation
undo_redo.undo();
let current = undo_redo.present().unwrap();
assert_eq!(current.current_page, "Settings");  // Navigation preserved
assert_eq!(current.todos.len(), 0);             // Todo reverted
```

**Behavior:**
- Updates the current state without adding a history entry
- If not at the most recent state (after undo), `replace_current()` truncates future history
- This maintains consistency: navigation forward would be confusing with stale future states
- Works with both `UndoRedo` and `PersistentUndoRedo`

**Use cases:**
- Navigation state in multi-page apps
- Cursor position in text editors
- Selected items in lists
- Expanded/collapsed states in tree views
- Zoom/pan state in canvas apps
- Search filters and sort orders

**Example: Text Editor**

```rust
#[derive(Clone)]
struct EditorState {
    document: Vector<String>,  // Undoable content
    cursor: (usize, usize),    // Ephemeral cursor position
}

impl Persistent for EditorState {}

// Type character (undoable)
editor.update(|state| {
    let mut new_doc = state.document.clone();
    new_doc.push_back("New line".to_string());
    EditorState {
        document: new_doc,
        cursor: (state.cursor.0 + 1, 0),
    }
}, "Type".to_string());

// Move cursor (not undoable)
editor.replace_current(|state| EditorState {
    cursor: (state.cursor.0, state.cursor.1 + 1),
    ..state.clone()
});
```

See the `todo_app` demo for a complete implementation using this pattern.

### Serialization Support

Export and restore history without requiring serde:

```rust
use undo_redo::sundo::{Actions, UndoRedo};

let mut undo_redo = UndoRedo::new();
undo_redo.push(42, "Initial".to_string());
undo_redo.update(|x| x + 10, "Add 10".to_string());

// Export to snapshot
let snapshot = undo_redo.export_snapshot();

// Access snapshot data
for (payload, description) in &snapshot.entries {
    println!("Entry: {} = {}", description, payload);
}

// You can now serialize the snapshot with any serialization library
// (serde, bincode, JSON, custom format, etc.)

// Restore from snapshot
let restored = UndoRedo::from_snapshot(snapshot);
assert_eq!(*restored.present().unwrap(), 52);
```

The `Snapshot` type includes:
- All history entries (payload + description pairs)
- Current pointer position
- Transaction state (if active)
- History limits configuration
- Memory tracking info (if enabled)

This gives you complete control over serialization without imposing a specific format.

### Inspecting History Without Moving

Use `peek_at()` to examine any history state without changing the current pointer:

```rust
use undo_redo::sundo::{Actions, UndoRedo};

let mut undo_redo = UndoRedo::new();
undo_redo.push(0, "Initial".to_string());
undo_redo.update(|x| x + 10, "Add 10".to_string());
undo_redo.update(|x| x + 20, "Add 20".to_string());

// Current state is 30
assert_eq!(*undo_redo.present().unwrap(), 30);

// Peek at earlier state without moving pointer
let previous = undo_redo.peek_at(1).unwrap();
assert_eq!(*previous, 10);

// Current state unchanged
assert_eq!(*undo_redo.present().unwrap(), 30);
assert_eq!(undo_redo.get_pointer(), 2);
```

**Use cases:**
- Displaying history preview in UI
- Computing deltas between states
- Comparing current state with previous versions
- Building audit logs
- Conflict detection

### Delta Computation Pattern

Combine `peek_at()` with custom logic to compute differences between states:

```rust
use undo_redo::sundo::{Actions, UndoRedo};

// Define a custom type with delta computation
#[derive(Clone, Debug, PartialEq)]
struct Document {
    content: String,
    version: u32,
}

struct Delta {
    from_version: u32,
    to_version: u32,
    changes: String,
}

fn compute_delta(from: &Document, to: &Document) -> Delta {
    Delta {
        from_version: from.version,
        to_version: to.version,
        changes: format!("Changed from '{}' to '{}'", from.content, to.content),
    }
}

// Usage
let mut undo_redo = UndoRedo::new();
undo_redo.push(Document { content: "Hello".to_string(), version: 1 }, "Initial".to_string());
undo_redo.update(
    |d| Document { content: format!("{} World", d.content), version: d.version + 1 },
    "Add World".to_string()
);

// Compute delta between consecutive states
let prev = undo_redo.peek_at(0).unwrap();
let curr = undo_redo.peek_at(1).unwrap();
let delta = compute_delta(&prev, &curr);

println!("Delta: {}", delta.changes);
// Prints: "Changed from 'Hello' to 'Hello World'"
```

**Advanced: Generate changelog from all deltas**

```rust
fn generate_changelog(undo_redo: &UndoRedo<Document>) -> Vec<Delta> {
    let mut deltas = Vec::new();
    
    for i in 0..undo_redo.get_history_len() - 1 {
        let from = undo_redo.peek_at(i).unwrap();
        let to = undo_redo.peek_at(i + 1).unwrap();
        deltas.push(compute_delta(&from, &to));
    }
    
    deltas
}
```

See the `delta-example` demo for a complete implementation.

## Example: Text Editor State

```rust
use im::Vector;
use undo_redo::sundo::{PersistentActions, PersistentUndoRedo, Persistent};

#[derive(Clone)]
struct Document {
    lines: Vector<String>,
    cursor: (usize, usize),
}

impl Persistent for Document {}

impl Document {
    fn insert_char(&self, line: usize, col: usize, ch: char) -> Self {
        let mut new_doc = self.clone();
        // Only the modified line is cloned internally by im::Vector
        let mut line_text = new_doc.lines[line].clone();
        line_text.insert(col, ch);
        new_doc.lines.set(line, line_text);
        new_doc.cursor = (line, col + 1);
        new_doc
    }
}

fn main() {
    let mut editor = PersistentUndoRedo::new();
    let doc = Document {
        lines: Vector::from(vec!["Hello".to_string()]),
        cursor: (0, 5),
    };
    
    editor.push(doc, "Initial".to_string());
    
    // Insert '!' - only stores the difference
    editor.update(
        |d| d.insert_char(0, 5, '!'),
        "Insert '!'".to_string()
    );
    
    // Efficient undo
    editor.undo();
}
```

## Performance Notes

- `PersistentUndoRedo` assumes cheap `clone()` (O(1) or O(log n))
- Don't use `PersistentUndoRedo` with expensive-to-clone types
- Transaction overhead is identical for both variants
- History navigation is O(1) for both variants
