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



## Migration Path

1. Start with `UndoRedo<T>` for prototyping
2. Switch to `im` types for your payload when memory becomes a concern
3. Change to `PersistentUndoRedo<T>` and add `impl Persistent` for maximum efficiency
4. Update method signatures: `Rc<T>` → `&T`

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
