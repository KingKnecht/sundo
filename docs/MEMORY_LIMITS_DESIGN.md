# Memory-Based History Limits Design

## Overview

Add optional memory-based limits to prevent unbounded history growth in applications with large state objects.

## Status

**Phase 1 (Count-based limits): ✅ IMPLEMENTED**  
- `UndoRedoBuilder` with `with_max_entries()` method
- `PersistentUndoRedoBuilder` with same functionality
- Automatic pruning of oldest entries when limit exceeded
- 10 comprehensive tests added
- Example demonstrating all use cases

**Phase 2 (Memory-based limits): ⚪ Not yet implemented**  
- Would add `MemoryFootprint` trait and memory-based tracking

## Design Goals

1. **Optional limits**: Both count-based and memory-based
2. **Automatic pruning**: Oldest entries are dropped when limits exceeded
3. **Accurate tracking**: Use trait for types to report their memory footprint
4. **Zero overhead**: Only track when limits are configured
5. **Persistent-aware**: Leverage structural sharing characteristics

## API Design

### 1. Memory Estimation Trait

```rust
/// Trait for types that can estimate their memory footprint
pub trait MemoryFootprint {
    /// Estimate memory usage in bytes (including heap allocations)
    fn memory_footprint(&self) -> usize;
}
```

### 2. Builder Pattern with Limits

```rust
pub struct UndoRedoBuilder<T> {
    initial_capacity: usize,
    max_entries: Option<usize>,      // Limit by count
    max_memory_bytes: Option<usize>, // Limit by memory
}

impl<T> UndoRedoBuilder<T> {
    pub fn new() -> Self;
    
    pub fn with_capacity(mut self, capacity: usize) -> Self;
    
    /// Limit history to N entries (oldest dropped first)
    pub fn with_max_entries(mut self, max: usize) -> Self;
    
    /// Limit history memory usage (in bytes)
    pub fn with_max_memory_bytes(mut self, max: usize) -> Self;
    
    /// Convenience: limit in megabytes
    pub fn with_max_memory_mb(self, mb: usize) -> Self {
        self.with_max_memory_bytes(mb * 1024 * 1024)
    }
    
    pub fn build(self) -> UndoRedo<T>;
}
```

### 3. Updated Structs

```rust
pub struct UndoRedo<Payload> {
    history: Vec<Entry<Payload>>,
    pointer: usize,
    transaction_start: Option<usize>,
    // New fields:
    max_entries: Option<usize>,
    max_memory_bytes: Option<usize>,
    current_memory_bytes: usize,  // Only tracked if max_memory_bytes.is_some()
}
```

## Usage Examples

### Example 1: Limit by Entry Count
```rust
let mut undo_redo = UndoRedoBuilder::new()
    .with_max_entries(100)
    .build();

// Keeps only the most recent 100 entries
for i in 0..200 {
    undo_redo.push(i, format!("Step {}", i));
}

assert!(undo_redo.get_history_len() <= 100);
```

### Example 2: Limit by Memory (Megabytes)
```rust
let mut undo_redo = UndoRedoBuilder::new()
    .with_max_memory_mb(100)  // 100 MB limit
    .build();

// Automatically prunes old entries when memory limit exceeded
```

### Example 3: Combined Limits
```rust
let mut undo_redo = UndoRedoBuilder::new()
    .with_max_entries(1000)
    .with_max_memory_mb(500)
    .build();

// Whichever limit is hit first triggers pruning
```

### Example 4: Infinite History (Default)
```rust
let mut undo_redo = UndoRedo::new();
// No limits - history grows unbounded (current behavior)
```

## Implementation Strategy

### Phase 1: Count-Based Limits (Simple)

```rust
impl<T> UndoRedo<T> {
    fn prune_if_needed(&mut self) {
        if let Some(max) = self.max_entries {
            while self.history.len() > max {
                // Remove oldest entry (index 0)
                self.history.remove(0);
                
                // Adjust pointer
                if self.pointer > 0 {
                    self.pointer -= 1;
                }
                
                // Adjust transaction_start if needed
                if let Some(ts) = self.transaction_start {
                    if ts > 0 {
                        self.transaction_start = Some(ts - 1);
                    }
                }
            }
        }
    }
    
    // Call prune_if_needed() after push(), update(), etc.
}
```

### Phase 2: Memory-Based Limits (Requires MemoryFootprint)

```rust
impl<T: MemoryFootprint> UndoRedo<T> {
    fn prune_by_memory(&mut self) {
        if let Some(max_mem) = self.max_memory_bytes {
            while self.current_memory_bytes > max_mem && self.history.len() > 1 {
                // Remove oldest entry
                if let Some(oldest) = self.history.first() {
                    let size = oldest.payload.memory_footprint() 
                             + oldest.description.len();
                    self.current_memory_bytes -= size;
                }
                
                self.history.remove(0);
                
                // Adjust pointer
                if self.pointer > 0 {
                    self.pointer -= 1;
                }
            }
        }
    }
}
```

## Default Implementations

### For `im::` Persistent Types

```rust
impl<T: Clone> MemoryFootprint for im::Vector<T> {
    fn memory_footprint(&self) -> usize {
        // im::Vector uses structural sharing, so this is approximate
        // Base overhead + (number of nodes * node size)
        std::mem::size_of::<Self>() + (self.len() * 8)
    }
}

impl<K: Clone, V: Clone> MemoryFootprint for im::HashMap<K, V> {
    fn memory_footprint(&self) -> usize {
        // Approximate: base + entries * (key + value + overhead)
        std::mem::size_of::<Self>() + (self.len() * 32)
    }
}
```

### For Custom Types

```rust
#[derive(Clone)]
struct AppState {
    users: HashMap<u64, String>,
    messages: Vector<String>,
}

impl MemoryFootprint for AppState {
    fn memory_footprint(&self) -> usize {
        self.users.memory_footprint() 
        + self.messages.memory_footprint()
    }
}
```

## Challenges & Solutions

### Challenge 1: Accurate Memory Measurement
**Problem**: Rust doesn't have reflection; measuring heap allocations is hard  
**Solution**: 
- Use trait-based estimation (good enough for limits)
- For persistent types, leverage structural sharing knowledge
- Allow users to implement trait for their types

### Challenge 2: Pruning with Active Undo/Redo
**Problem**: What if user undoes past the pruned entries?  
**Solution**: 
- Only prune entries older than current pointer position
- OR: Accept that pruned entries are gone (document clearly)
- Option: Keep minimum entries even if over memory limit

### Challenge 3: Performance Impact
**Problem**: Tracking memory adds overhead  
**Solution**: 
- Only track when `max_memory_bytes.is_some()`
- Use efficient estimation (no deep traversal)
- Cache footprint calculations where possible

### Challenge 4: Pointer Invalidation
**Problem**: Removing entries shifts indices  
**Solution**: 
- Always remove from front (oldest first)
- Adjust pointer and transaction_start accordingly
- Maintain invariants carefully

## Configuration Options

```rust
pub struct LimitConfig {
    /// Maximum history entries (None = unlimited)
    pub max_entries: Option<usize>,
    
    /// Maximum memory usage in bytes (None = unlimited)
    pub max_memory_bytes: Option<usize>,
    
    /// Minimum entries to keep even if over memory limit
    pub min_entries_to_keep: usize,  // Default: 1
    
    /// Keep entries newer than pointer (can't undo past pruned entries)
    pub preserve_current_path: bool,  // Default: true
}
```

## Testing Considerations

```rust
#[test]
fn test_max_entries_pruning() {
    let mut undo_redo = UndoRedoBuilder::new()
        .with_max_entries(5)
        .build();
    
    for i in 0..10 {
        undo_redo.push(i, format!("{}", i));
    }
    
    assert_eq!(undo_redo.get_history_len(), 5);
    assert_eq!(*undo_redo.present().unwrap(), 9);
}

#[test]
fn test_memory_limit_pruning() {
    let mut undo_redo = UndoRedoBuilder::new()
        .with_max_memory_bytes(1000)
        .build();
    
    // Add entries until memory limit hit
    // Verify oldest entries are pruned
}
```

## Migration Path

1. **Phase 1**: Add builder pattern with count-based limits (no breaking changes)
2. **Phase 2**: Add `MemoryFootprint` trait and memory-based limits (opt-in)
3. **Phase 3**: Implement for all `im::` types
4. **Documentation**: Clear examples and best practices

## Open Questions

1. Should we expose `current_memory_bytes` as a query method?
2. What's the default behavior: unlimited or a sensible default limit?
3. Should we emit warnings/logs when pruning occurs?
4. Do we need a callback/hook for when entries are pruned?
5. Should memory estimation be recursive (deep) or shallow?

## Summary

Memory-based history limits provide:

✅ **Control**: Prevent unbounded memory growth  
✅ **Flexibility**: Choose count-based, memory-based, or both  
✅ **Transparency**: Users can implement trait for accurate estimation  
✅ **Performance**: Only track when configured  
✅ **Safety**: Automatic pruning prevents OOM situations  

This feature would be especially valuable for:
- Long-running applications (editors, graphic tools)
- Mobile/embedded platforms with limited memory
- Applications with large state objects (CAD, data analysis)
- Multi-document interfaces sharing memory budget
