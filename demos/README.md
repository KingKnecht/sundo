# Sundo Demos

This directory contains standalone demonstration applications showing various features of the sundo library.

## Running Demos

Each demo is a standalone Cargo project. Navigate to its directory and run:

```bash
cd basic-example
cargo run --release
```

## Available Demos

### basic-example
Compares standard `UndoRedo<T>` with `PersistentUndoRedo<T>` for persistent data structures.

**Topics covered:**
- Standard undo/redo with Vec
- Persistent undo/redo with im::Vector
- im::HashMap usage
- Memory efficiency comparison

**Run:** `cd basic-example && cargo run`

---

### persistent-example
Deep dive into persistent data structures with detailed explanations.

**Topics covered:**
- im::Vector operations
- im::HashMap operations
- Structural sharing benefits
- Performance characteristics

**Run:** `cd persistent-example && cargo run`

---

### scoped-transaction
Demonstrates transaction patterns with automatic rollback.

**Topics covered:**
- RAII transactions
- Explicit commit
- Auto-rollback on error
- Exception-safe patterns
- Nested scopes

**Run:** `cd scoped-transaction && cargo run`

---

### builder-pattern
Shows history limit configuration and builder pattern usage.

**Topics covered:**
- UndoRedoBuilder API
- Count-based history limits
- Automatic pruning
- Use cases (editors, apps, embedded)
- Memory management

**Run:** `cd builder-pattern && cargo run`

---

## Future Demos

These demos are planned for future development:

- **todolist-app** - Complete todo list application using sundo
- **text-editor** - Simple text editor with undo/redo
- **canvas-app** - Graphics canvas with state management

## Contributing

Have an idea for a demo? Please open an issue or submit a PR!
