# Persistent Data Structures Integration

## Overview

The `Persistent` trait is a marker trait that identifies types using **structural sharing** - data structures that are cheap to clone because they share internal structure between versions.

## What is the Persistent Trait?

```rust
pub trait Persistent: Clone {}
```

This trait marks types that:
- Use structural sharing internally (e.g., `im::Vector`, `im::HashMap`)
- Are cheap to clone (O(1) or O(log n) instead of O(n))
- Share unchanged data between versions

## Automatic Implementations

The trait is automatically implemented for common persistent collections from the [`im`](https://crates.io/crates/im) crate:

```rust
impl<T: Clone> Persistent for im::Vector<T> {}
impl<K: Clone, V: Clone> Persistent for im::HashMap<K, V> {}
impl<K: Clone, V: Clone> Persistent for im::OrdMap<K, V> {}
impl<T: Clone> Persistent for im::HashSet<T> {}
impl<T: Clone> Persistent for im::OrdSet<T> {}
```

## Usage

### For Built-in Persistent Types

No action needed - they're already marked as `Persistent`:

```rust
use im::Vector;
use undo_redo::sundo::{Actions, UndoRedo};

let mut undo_redo = UndoRedo::new();
let vec = Vector::from(vec![1, 2, 3]);
undo_redo.push(vec, "Initial".to_string());
```

### For Custom Types

Mark your type as `Persistent` if it's composed of persistent data structures:

```rust
use im::{Vector, HashMap};
use undo_redo::sundo::Persistent;

#[derive(Clone)]
struct AppState {
    users: HashMap<u64, String>,
    messages: Vector<String>,
}

// Mark as Persistent since it only contains persistent types
impl Persistent for AppState {}
```

### Helper Methods Pattern

Create helper methods that leverage structural sharing:

```rust
impl AppState {
    fn add_user(&self, id: u64, name: String) -> Self {
        Self {
            users: self.users.update(id, name),
            messages: self.messages.clone(),  // Cheap! Just ref bump
        }
    }
    
    fn add_message(&self, msg: String) -> Self {
        let mut messages = self.messages.clone();
        messages.push_back(msg);
        Self {
            users: self.users.clone(),  // Cheap!
            messages,
        }
    }
}
```

## Benefits

### Memory Efficiency

With persistent types + undo/redo, you get massive memory savings:

```
Standard Vec (10 versions): ~500 MB
im::Vector (10 versions):   ~61 MB
Savings:                    ~89%
```

See [`mem_performance_test.rs`](../src/mem_performance_test.rs) for detailed benchmarks.

### Performance

- **Clone operations**: O(1) or O(log n) instead of O(n)
- **Memory sharing**: Unchanged parts are shared, not copied
- **Cache efficiency**: Better locality than full deep copies

### Thread Safety

Persistent data structures are naturally thread-safe because they're immutable.

## Future Optimizations

The `Persistent` trait enables future optimizations:

1. **No Rc wrapping**: Skip `Rc` for types that already share internally
2. **Specialized storage**: Different storage strategies for persistent vs regular types
3. **Better algorithms**: Optimizations that leverage cheap cloning

## Complete Example

```rust
use undo_redo::sundo::{Actions, UndoRedo, Persistent};
use im::{Vector, HashMap};

#[derive(Clone)]
struct Document {
    lines: Vector<String>,
    marks: HashMap<String, usize>,
}

impl Persistent for Document {}

impl Document {
    fn insert_line(&self, pos: usize, text: String) -> Self {
        let mut lines = self.lines.clone();
        lines.insert(pos, text);
        Self {
            lines,
            marks: self.marks.clone(),  // Cheap!
        }
    }
}

fn main() {
    let mut undo_redo = UndoRedo::new();
    
    let doc = Document {
        lines: Vector::new(),
        marks: HashMap::new(),
    };
    undo_redo.push(doc, "New document".to_string());
    
    // Each modification shares most of the data
    undo_redo.update(
        |d| d.insert_line(0, "Hello".to_string()),
        "Insert line".to_string()
    );
    
    undo_redo.undo(); // Efficient!
}
```

## When NOT to Use

Don't mark as `Persistent` if your type:
- Contains `Vec`, `HashMap`, or other standard collections (expensive to clone)
- Has large owned data that isn't shared
- Doesn't benefit from the marker (no planned optimizations yet)

## See Also

- [im crate documentation](https://docs.rs/im)
- [Persistent example](../examples/persistent_example.rs)
- [Memory performance tests](../src/mem_performance_test.rs)
- [Persistent integration examples](../src/persistent_integration_example.rs)

## Comparison with Standard Collections

| Operation | Vec<T> | im::Vector<T> |
|-----------|--------|---------------|
| Clone all | O(n) | O(1)* |
| Push | O(1) amortized | O(1)* |
| Access | O(1) | O(log n)* |
| Memory | Compact | More overhead |

*With structural sharing - only changed parts copied

**Bottom line**: For undo/redo systems with large collections, persistent data structures + the `Persistent` trait can reduce memory usage by 80-90% while maintaining good performance.
