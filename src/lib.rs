//#[allow(dead_code)] //Do not complain about unused structs etc.

pub mod sundo {
    use std::rc::Rc;

    /// Marker trait for types that use structural sharing (persistent data structures).
    /// 
    /// Types implementing this trait are cheap to clone because they share structure
    /// internally (e.g., `im::Vector`, `im::HashMap`). This allows the undo/redo system
    /// to potentially optimize storage and avoid unnecessary `Rc` wrapping in the future.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use im::Vector;
    /// 
    /// // im::Vector already implements Persistent
    /// let mut undo_redo = UndoRedo::new();
    /// let data = Vector::from(vec![1, 2, 3]);
    /// undo_redo.push(data, "Initial".to_string());
    /// ```
    /// 
    /// For custom types that contain persistent data structures:
    /// 
    /// ```ignore
    /// use im::{Vector, HashMap};
    /// 
    /// #[derive(Clone)]
    /// struct AppState {
    ///     users: HashMap<u64, String>,
    ///     messages: Vector<String>,
    /// }
    /// 
    /// // Mark your type as persistent if it's cheap to clone
    /// impl Persistent for AppState {}
    /// ```
    pub trait Persistent: Clone {}

    // Blanket implementations for im crate persistent data structures
    impl<T: Clone> Persistent for im::Vector<T> {}
    impl<K: Clone, V: Clone> Persistent for im::HashMap<K, V> {}
    impl<K: Clone, V: Clone> Persistent for im::OrdMap<K, V> {}
    impl<T: Clone> Persistent for im::HashSet<T> {}
    impl<T: Clone> Persistent for im::OrdSet<T> {}

    // ============================================================================
    // Standard UndoRedo with Rc wrapping (for any cloneable type)
    // ============================================================================

    struct Entry<Payload> {
        payload: Rc<Payload>,
        description: String,
    }

    pub struct UndoRedo<Payload> {
        history: Vec<Entry<Payload>>,
        pointer: usize,
        transaction_start: Option<usize>,
        max_entries: Option<usize>,
    }

    // ============================================================================
    // Optimized PersistentUndoRedo (no Rc wrapping for Persistent types)
    // ============================================================================

    /// Optimized undo/redo for persistent data structures.
    /// 
    /// Unlike `UndoRedo` which wraps payloads in `Rc`, `PersistentUndoRedo` stores
    /// persistent types directly. This avoids double indirection since persistent
    /// types already use structural sharing internally.
    /// 
    /// # Benefits
    /// 
    /// - No `Rc` overhead (~16 bytes per entry)
    /// - One less heap allocation per entry
    /// - Better cache locality
    /// - Simpler API (returns `&T` instead of `Rc<T>`)
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use im::Vector;
    /// use undo_redo::sundo::{PersistentActions, PersistentUndoRedo};
    /// 
    /// let mut undo_redo = PersistentUndoRedo::new();
    /// let vec = Vector::from(vec![1, 2, 3]);
    /// undo_redo.push(vec, "Initial".to_string());
    /// 
    /// // Returns &Vector<i32> instead of Rc<Vector<i32>>
    /// let current = undo_redo.present().unwrap();
    /// ```
    struct PersistentEntry<Payload> {
        payload: Payload,  // Store directly, no Rc!
        description: String,
    }

    pub struct PersistentUndoRedo<Payload: Persistent> {
        history: Vec<PersistentEntry<Payload>>,
        pointer: usize,
        transaction_start: Option<usize>,
        max_entries: Option<usize>,
    }

    /// Actions trait for persistent data structures.
    /// 
    /// Similar to `Actions` but optimized for types implementing `Persistent`.
    /// Returns references instead of `Rc` since cloning is cheap.
    pub trait PersistentActions<Payload: Persistent> {
        fn new() -> Self;
        fn push(&mut self, value: Payload, description: String) -> &Payload;
        fn can_undo(&self) -> bool;
        fn can_redo(&self) -> bool;
        fn undo(&mut self) -> Option<&Payload>;
        fn redo(&mut self) -> Option<&Payload>;
        fn present(&self) -> Option<&Payload>;
        fn update<F>(&mut self, f: F, description: String) -> Option<&Payload>
        where
            F: FnOnce(&Payload) -> Payload;
        fn start_transaction(&mut self);
        fn end_transaction(&mut self, description: String) -> bool;
        fn cancel_transaction(&mut self) -> bool;
        fn get_history(&self) -> Option<Vec<HistoryEntry>>;
        fn go_to(&mut self, index: usize) -> Option<&Payload>;
        fn get_pointer(&self) -> usize;
        fn get_history_len(&self) -> usize;
        
        /// Get entry at specific index without moving pointer.
        /// 
        /// Useful for comparing states or computing deltas.
        fn peek_at(&self, index: usize) -> Option<&Payload>;
        
        /// Export history as a snapshot for serialization.
        fn export_snapshot(&self) -> PersistentSnapshot<Payload>;
        
        /// Reconstruct from a snapshot.
        fn from_snapshot(snapshot: PersistentSnapshot<Payload>) -> Self;
    }

    // ============================================================================
    // Shared types
    // ============================================================================

    #[derive(Debug, PartialEq)]
    pub enum Tenses {
        Past,
        Present,
        Future        
    }

    #[derive(Debug)]
    pub struct HistoryEntry{
        pub description: String,
        pub tense: Tenses,
        pub pointer : usize
    }

    // ============================================================================
    // Serialization Support
    // ============================================================================

    /// Snapshot of UndoRedo history for serialization.
    /// 
    /// Extract this from an UndoRedo instance, serialize the payloads however you like,
    /// then reconstruct the UndoRedo from the snapshot.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// // Export
    /// let snapshot = undo_redo.export_snapshot();
    /// let serialized: Vec<String> = snapshot.entries
    ///     .iter()
    ///     .map(|(payload, desc)| format!("{},{}", payload, desc))
    ///     .collect();
    /// 
    /// // Import
    /// let entries: Vec<(i32, String)> = serialized
    ///     .iter()
    ///     .map(|s| /* parse... */)
    ///     .collect();
    /// let restored = UndoRedo::from_snapshot(entries, snapshot.pointer, snapshot.max_entries);
    /// ```
    #[derive(Debug, Clone)]
    pub struct Snapshot<T> {
        /// All history entries as (payload, description) pairs
        pub entries: Vec<(Rc<T>, String)>,
        /// Current position in history
        pub pointer: usize,
        /// Active transaction start position, if any
        pub transaction_start: Option<usize>,
        /// Maximum entries limit, if any
        pub max_entries: Option<usize>,
    }

    /// Snapshot for PersistentUndoRedo (no Rc wrapping).
    #[derive(Debug, Clone)]
    pub struct PersistentSnapshot<T: Persistent> {
        /// All history entries as (payload, description) pairs
        pub entries: Vec<(T, String)>,
        /// Current position in history
        pub pointer: usize,
        /// Active transaction start position, if any
        pub transaction_start: Option<usize>,
        /// Maximum entries limit, if any
        pub max_entries: Option<usize>,
    }

    pub trait Actions<Payload> {
        fn new() -> Self;
        fn push(&mut self, value: Payload, description: String) -> Rc<Payload>;
        fn can_undo(&self) -> bool;
        fn can_redo(&self) -> bool;
        fn undo(&mut self) -> Option<Rc<Payload>>;
        fn redo(&mut self) -> Option<Rc<Payload>>;
        fn present(&self) -> Option<Rc<Payload>>;
        fn update<F>(&mut self, f: F, description: String) -> Option<Rc<Payload>>
        where
            F: FnOnce(&Payload) -> Payload;
        fn start_transaction(&mut self);
        fn end_transaction(&mut self, description: String) -> bool;
        fn cancel_transaction(&mut self) -> bool;
        fn get_history(&self) -> Option<Vec<HistoryEntry>>;
        fn go_to(&mut self, index: usize) -> Option<Rc<Payload>>;
        fn get_pointer(&self) -> usize;
        fn get_history_len(&self) -> usize;
        
        /// Get entry at specific index without moving pointer.
        /// 
        /// Useful for comparing states or computing deltas.
        fn peek_at(&self, index: usize) -> Option<Rc<Payload>>;
        
        /// Export history as a snapshot for serialization.
        /// 
        /// Users can then serialize the payloads with their preferred method.
        fn export_snapshot(&self) -> Snapshot<Payload>;
        
        /// Reconstruct UndoRedo from a snapshot.
        /// 
        /// This is a static method to create a new instance from exported data.
        fn from_snapshot(snapshot: Snapshot<Payload>) -> Self;
    } 

    impl<Payload> Actions<Payload> for UndoRedo<Payload> {
        fn new() -> Self {
            Self {
                history: Vec::with_capacity(100),
                pointer: 0,
                transaction_start: None,
                max_entries: None,
            }
        }

        fn push(&mut self, payload: Payload, description: String) -> Rc<Payload> {
            // If not at the end, truncate future history
            if !self.history.is_empty() && self.pointer + 1 < self.history.len() {
                self.history.truncate(self.pointer + 1);
            }
            
            let rc_payload = Rc::new(payload);
            self.history.push(Entry {
                payload: Rc::clone(&rc_payload),
                description,
            });
            self.pointer = self.history.len() - 1;

            self.prune_if_needed();
            rc_payload
        }

        fn can_undo(&self) -> bool {
            self.pointer > 0
        }

        fn can_redo(&self) -> bool {
            !self.history.is_empty() && self.pointer < self.history.len() - 1
        }

        fn undo(&mut self) -> Option<Rc<Payload>> {
            if !self.can_undo() {
                return None;
            }
            
            self.pointer -= 1;
            self.history.get(self.pointer).map(|e| Rc::clone(&e.payload))
        }

        fn redo(&mut self) -> Option<Rc<Payload>> {
            if !self.can_redo() {
                return None;
            }
            
            self.pointer += 1;
            self.history.get(self.pointer).map(|e| Rc::clone(&e.payload))
        }
        
        fn present(&self) -> Option<Rc<Payload>> {
            self.history.get(self.pointer).map(|e| Rc::clone(&e.payload))
        }
        
        fn update<F>(&mut self, f: F, description: String) -> Option<Rc<Payload>>
        where
            F: FnOnce(&Payload) -> Payload,
        {
            let current = self.present()?;
            let new_payload = f(&current);
            Some(self.push(new_payload, description))
        }
        
        fn start_transaction(&mut self) {
            if self.transaction_start.is_none() {
                self.transaction_start = Some(self.pointer);
            }
        }
        
        fn end_transaction(&mut self, description: String) -> bool {
            if let Some(start_pointer) = self.transaction_start.take() {
                if self.pointer > start_pointer {
                    // Get the final state (current pointer)
                    let final_state = self.history[self.pointer].payload.clone();
                    
                    // Truncate to just after transaction start
                    self.history.truncate(start_pointer + 1);
                    
                    // Replace with single entry containing final state
                    self.history[start_pointer] = Entry {
                        payload: final_state,
                        description,
                    };
                    
                    self.pointer = start_pointer;
                }
                true
            } else {
                false
            }
        }
        
        fn cancel_transaction(&mut self) -> bool {
            if let Some(start_pointer) = self.transaction_start.take() {
                // Truncate all changes made during transaction
                self.history.truncate(start_pointer + 1);
                self.pointer = start_pointer;
                true
            } else {
                false
            }
        }
        
        fn get_history(&self) -> Option<Vec<HistoryEntry>> {
            if self.history.is_empty() {
                return None;
            }
            
            let entries: Vec<HistoryEntry> = self.history
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    let tense = if index < self.pointer {
                        Tenses::Past
                    } else if index == self.pointer {
                        Tenses::Present
                    } else {
                        Tenses::Future
                    };
                    
                    HistoryEntry {
                        description: entry.description.clone(),
                        tense,
                        pointer: index,
                    }
                })
                .collect();
            
            Some(entries)
        }
        
        fn go_to(&mut self, index: usize) -> Option<Rc<Payload>> {
            if index > self.history.len() - 1 {
                return None;
            }
            
            self.pointer = index;
            self.history.get(self.pointer).map(|e| Rc::clone(&e.payload))   
        }
        
        fn get_pointer(&self) -> usize {
            self.pointer
        }
        
        fn get_history_len(&self) -> usize {
            self.history.len()
        }
        
        fn peek_at(&self, index: usize) -> Option<Rc<Payload>> {
            self.history.get(index).map(|e| Rc::clone(&e.payload))
        }
        
        fn export_snapshot(&self) -> Snapshot<Payload> {
            Snapshot {
                entries: self.history
                    .iter()
                    .map(|e| (Rc::clone(&e.payload), e.description.clone()))
                    .collect(),
                pointer: self.pointer,
                transaction_start: self.transaction_start,
                max_entries: self.max_entries,
            }
        }
        
        fn from_snapshot(snapshot: Snapshot<Payload>) -> Self {
            let history = snapshot.entries
                .into_iter()
                .map(|(payload, description)| Entry { payload, description })
                .collect();
            
            Self {
                history,
                pointer: snapshot.pointer,
                transaction_start: snapshot.transaction_start,
                max_entries: snapshot.max_entries,
            }
        }
        
    }

    // ============================================================================
    // PersistentActions Implementation - Optimized for structural sharing
    // ============================================================================

    impl<Payload: Persistent> PersistentActions<Payload> for PersistentUndoRedo<Payload> {
        fn new() -> Self {
            Self {
                history: Vec::with_capacity(100),
                pointer: 0,
                transaction_start: None,
                max_entries: None,
            }
        }

        fn push(&mut self, payload: Payload, description: String) -> &Payload {
            // If not at the end, truncate future history
            if !self.history.is_empty() && self.pointer + 1 < self.history.len() {
                self.history.truncate(self.pointer + 1);
            }
            
            self.history.push(PersistentEntry {
                payload,  // Store directly, no Rc wrapping!
                description,
            });
            self.pointer = self.history.len() - 1;

            self.prune_if_needed();
            // Return reference to the stored payload
            &self.history[self.pointer].payload
        }

        fn can_undo(&self) -> bool {
            self.pointer > 0
        }

        fn can_redo(&self) -> bool {
            !self.history.is_empty() && self.pointer < self.history.len() - 1
        }

        fn undo(&mut self) -> Option<&Payload> {
            if !self.can_undo() {
                return None;
            }
            
            self.pointer -= 1;
            self.history.get(self.pointer).map(|e| &e.payload)
        }

        fn redo(&mut self) -> Option<&Payload> {
            if !self.can_redo() {
                return None;
            }
            
            self.pointer += 1;
            self.history.get(self.pointer).map(|e| &e.payload)
        }
        
        fn present(&self) -> Option<&Payload> {
            self.history.get(self.pointer).map(|e| &e.payload)
        }
        
        fn update<F>(&mut self, f: F, description: String) -> Option<&Payload>
        where
            F: FnOnce(&Payload) -> Payload,
        {
            let current = self.present()?;
            let new_payload = f(current);  // Cheap clone via structural sharing!
            Some(self.push(new_payload, description))
        }
        
        fn start_transaction(&mut self) {
            if self.transaction_start.is_none() {
                self.transaction_start = Some(self.pointer);
            }
        }
        
        fn end_transaction(&mut self, description: String) -> bool {
            if let Some(start_pointer) = self.transaction_start.take() {
                if self.pointer > start_pointer {
                    // Get the final state (current pointer)
                    let final_state = self.history[self.pointer].payload.clone();
                    
                    // Truncate to just after transaction start
                    self.history.truncate(start_pointer + 1);
                    
                    // Replace with single entry containing final state
                    self.history[start_pointer] = PersistentEntry {
                        payload: final_state,
                        description,
                    };
                    
                    self.pointer = start_pointer;
                }
                true
            } else {
                false
            }
        }
        
        fn cancel_transaction(&mut self) -> bool {
            if let Some(start_pointer) = self.transaction_start.take() {
                // Truncate all changes made during transaction
                self.history.truncate(start_pointer + 1);
                self.pointer = start_pointer;
                true
            } else {
                false
            }
        }
        
        fn get_history(&self) -> Option<Vec<HistoryEntry>> {
            if self.history.is_empty() {
                return None;
            }
            
            let entries: Vec<HistoryEntry> = self.history
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    let tense = if index < self.pointer {
                        Tenses::Past
                    } else if index == self.pointer {
                        Tenses::Present
                    } else {
                        Tenses::Future
                    };
                    
                    HistoryEntry {
                        description: entry.description.clone(),
                        tense,
                        pointer: index,
                    }
                })
                .collect();
            
            Some(entries)
        }
        
        fn go_to(&mut self, index: usize) -> Option<&Payload> {
            if index > self.history.len() - 1 {
                return None;
            }
            
            self.pointer = index;
            self.history.get(self.pointer).map(|e| &e.payload)
        }
        
        fn get_pointer(&self) -> usize {
            self.pointer
        }
        
        fn get_history_len(&self) -> usize {
            self.history.len()
        }
        
        fn peek_at(&self, index: usize) -> Option<&Payload> {
            self.history.get(index).map(|e| &e.payload)
        }
        
        fn export_snapshot(&self) -> PersistentSnapshot<Payload> {
            PersistentSnapshot {
                entries: self.history
                    .iter()
                    .map(|e| (e.payload.clone(), e.description.clone()))
                    .collect(),
                pointer: self.pointer,
                transaction_start: self.transaction_start,
                max_entries: self.max_entries,
            }
        }
        
        fn from_snapshot(snapshot: PersistentSnapshot<Payload>) -> Self {
            let history = snapshot.entries
                .into_iter()
                .map(|(payload, description)| PersistentEntry { payload, description })
                .collect();
            
            Self {
                history,
                pointer: snapshot.pointer,
                transaction_start: snapshot.transaction_start,
                max_entries: snapshot.max_entries,
            }
        }
    }

    // ============================================================================
    // Scoped Transaction (RAII Pattern)
    // ============================================================================

    /// RAII transaction wrapper for UndoRedo.
    /// 
    /// Automatically calls `start_transaction()` on creation and either
    /// `end_transaction()` on explicit `commit()` or `cancel_transaction()`
    /// if dropped without committing.
    /// 
    /// This provides exception-safe transaction handling - if your code panics
    /// or returns early with `?`, the transaction will automatically roll back.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use undo_redo::sundo::{Actions, UndoRedo, ScopedTransaction};
    /// 
    /// let mut undo_redo = UndoRedo::new();
    /// undo_redo.push(0, "Initial".to_string());
    /// 
    /// // Transaction that commits
    /// {
    ///     let mut tx = ScopedTransaction::begin(&mut undo_redo, "Add 10");
    ///     tx.get().update(|x| x + 5, "temp".to_string());
    ///     tx.get().update(|x| x + 5, "temp".to_string());
    ///     tx.commit();  // Explicitly commit
    /// }
    /// 
    /// // Transaction that auto-rolls back
    /// {
    ///     let mut tx = ScopedTransaction::begin(&mut undo_redo, "This fails");
    ///     tx.get().update(|x| x + 100, "temp".to_string());
    ///     // Forgot to commit, or early return/panic -> auto-rollback!
    /// }
    /// ```
    pub struct ScopedTransaction<'a, T> {
        undo_redo: &'a mut UndoRedo<T>,
        description: String,
        committed: bool,
    }

    impl<'a, T> ScopedTransaction<'a, T> {
        /// Begin a new transaction.
        /// 
        /// Calls `start_transaction()` on the provided UndoRedo instance.
        pub fn begin(undo_redo: &'a mut UndoRedo<T>, description: impl Into<String>) -> Self {
            undo_redo.start_transaction();
            Self {
                undo_redo,
                description: description.into(),
                committed: false,
            }
        }
        
        /// Commit the transaction with the provided description.
        /// 
        /// Calls `end_transaction()` and marks the transaction as committed,
        /// preventing the Drop implementation from rolling back.
        pub fn commit(mut self) {
            self.undo_redo.end_transaction(self.description.clone());
            self.committed = true;
        }
        
        /// Get mutable access to the wrapped UndoRedo.
        /// 
        /// Use this to perform operations within the transaction.
        pub fn get(&mut self) -> &mut UndoRedo<T> {
            self.undo_redo
        }
    }

    impl<'a, T> Drop for ScopedTransaction<'a, T> {
        /// Automatically rolls back if not committed.
        fn drop(&mut self) {
            if !self.committed {
                self.undo_redo.cancel_transaction();
            }
        }
    }

    /// RAII transaction wrapper for PersistentUndoRedo.
    /// 
    /// Same as `ScopedTransaction` but for persistent data structures.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use im::Vector;
    /// use undo_redo::sundo::{PersistentActions, PersistentUndoRedo, PersistentScopedTransaction};
    /// 
    /// let mut undo_redo = PersistentUndoRedo::new();
    /// undo_redo.push(Vector::new(), "Initial".to_string());
    /// 
    /// {
    ///     let mut tx = PersistentScopedTransaction::begin(&mut undo_redo, "Add items");
    ///     tx.get().update(|v| v.push_back(1), "temp".to_string());
    ///     tx.get().update(|v| v.push_back(2), "temp".to_string());
    ///     tx.commit();
    /// }
    /// ```
    pub struct PersistentScopedTransaction<'a, T: Persistent> {
        undo_redo: &'a mut PersistentUndoRedo<T>,
        description: String,
        committed: bool,
    }

    impl<'a, T: Persistent> PersistentScopedTransaction<'a, T> {
        /// Begin a new transaction.
        pub fn begin(undo_redo: &'a mut PersistentUndoRedo<T>, description: impl Into<String>) -> Self {
            undo_redo.start_transaction();
            Self {
                undo_redo,
                description: description.into(),
                committed: false,
            }
        }
        
        /// Commit the transaction with the provided description.
        pub fn commit(mut self) {
            self.undo_redo.end_transaction(self.description.clone());
            self.committed = true;
        }
        
        /// Get mutable access to the wrapped PersistentUndoRedo.
        pub fn get(&mut self) -> &mut PersistentUndoRedo<T> {
            self.undo_redo
        }
    }

    impl<'a, T: Persistent> Drop for PersistentScopedTransaction<'a, T> {
        /// Automatically rolls back if not committed.
        fn drop(&mut self) {
            if !self.committed {
                self.undo_redo.cancel_transaction();
            }
        }
    }

    // ============================================================================
    // Builder Pattern for UndoRedo with History Limits
    // ============================================================================

    /// Builder for constructing UndoRedo with optional configuration.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use undo_redo::sundo::{Actions, UndoRedoBuilder};
    /// 
    /// // With default settings (unlimited history)
    /// let undo_redo = UndoRedoBuilder::new().build();
    /// 
    /// // With count-based limit
    /// let undo_redo = UndoRedoBuilder::new()
    ///     .with_max_entries(100)
    ///     .build();
    /// 
    /// // With custom capacity
    /// let undo_redo = UndoRedoBuilder::new()
    ///     .with_capacity(1000)
    ///     .with_max_entries(500)
    ///     .build();
    /// ```
    pub struct UndoRedoBuilder<T> {
        initial_capacity: usize,
        max_entries: Option<usize>,
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> UndoRedoBuilder<T> {
        /// Create a new builder with default settings.
        pub fn new() -> Self {
            Self {
                initial_capacity: 100,
                max_entries: None,
                _phantom: std::marker::PhantomData,
            }
        }
        
        /// Set the initial capacity for the history vector.
        /// 
        /// This is a performance optimization to reduce allocations.
        pub fn with_capacity(mut self, capacity: usize) -> Self {
            self.initial_capacity = capacity;
            self
        }
        
        /// Set the maximum number of history entries to keep.
        /// 
        /// When the history exceeds this limit, the oldest entries are
        /// automatically removed. The current state is always preserved.
        /// 
        /// Pass `None` for unlimited history (default).
        pub fn with_max_entries(mut self, max: usize) -> Self {
            self.max_entries = Some(max);
            self
        }
        
        /// Build the UndoRedo instance.
        pub fn build(self) -> UndoRedo<T> {
            UndoRedo {
                history: Vec::with_capacity(self.initial_capacity),
                pointer: 0,
                transaction_start: None,
                max_entries: self.max_entries,
            }
        }
    }

    impl<T> Default for UndoRedoBuilder<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Builder for constructing PersistentUndoRedo with optional configuration.
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// use im::Vector;
    /// use undo_redo::sundo::{PersistentActions, PersistentUndoRedoBuilder};
    /// 
    /// let undo_redo = PersistentUndoRedoBuilder::new()
    ///     .with_max_entries(100)
    ///     .build();
    /// ```
    pub struct PersistentUndoRedoBuilder<T: Persistent> {
        initial_capacity: usize,
        max_entries: Option<usize>,
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T: Persistent> PersistentUndoRedoBuilder<T> {
        /// Create a new builder with default settings.
        pub fn new() -> Self {
            Self {
                initial_capacity: 100,
                max_entries: None,
                _phantom: std::marker::PhantomData,
            }
        }
        
        /// Set the initial capacity for the history vector.
        pub fn with_capacity(mut self, capacity: usize) -> Self {
            self.initial_capacity = capacity;
            self
        }
        
        /// Set the maximum number of history entries to keep.
        pub fn with_max_entries(mut self, max: usize) -> Self {
            self.max_entries = Some(max);
            self
        }
        
        /// Build the PersistentUndoRedo instance.
        pub fn build(self) -> PersistentUndoRedo<T> {
            PersistentUndoRedo {
                history: Vec::with_capacity(self.initial_capacity),
                pointer: 0,
                transaction_start: None,
                max_entries: self.max_entries,
            }
        }
    }

    impl<T: Persistent> Default for PersistentUndoRedoBuilder<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    // ============================================================================
    // Pruning Logic for History Limits
    // ============================================================================

    impl<T> UndoRedo<T> {
        /// Prune history to stay within max_entries limit.
        /// 
        /// Removes oldest entries first, preserving the current state.
        /// Adjusts pointer and transaction_start to maintain consistency.
        fn prune_if_needed(&mut self) {
            if let Some(max) = self.max_entries {
                while self.history.len() > max && self.history.len() > 1 {
                    // Remove oldest entry (index 0)
                    self.history.remove(0);
                    
                    // Adjust pointer (we're always at the end after push/update)
                    if self.pointer > 0 {
                        self.pointer -= 1;
                    }
                    
                    // Adjust transaction_start if active
                    if let Some(ts) = self.transaction_start {
                        if ts > 0 {
                            self.transaction_start = Some(ts - 1);
                        } else {
                            // Transaction started at index 0, which we just removed
                            // This shouldn't happen in normal use, but be safe
                            self.transaction_start = Some(0);
                        }
                    }
                }
            }
        }
    }

    impl<T: Persistent> PersistentUndoRedo<T> {
        /// Prune history to stay within max_entries limit.
        /// 
        /// Removes oldest entries first, preserving the current state.
        fn prune_if_needed(&mut self) {
            if let Some(max) = self.max_entries {
                while self.history.len() > max && self.history.len() > 1 {
                    // Remove oldest entry (index 0)
                    self.history.remove(0);
                    
                    // Adjust pointer
                    if self.pointer > 0 {
                        self.pointer -= 1;
                    }
                    
                    // Adjust transaction_start if active
                    if let Some(ts) = self.transaction_start {
                        if ts > 0 {
                            self.transaction_start = Some(ts - 1);
                        } else {
                            self.transaction_start = Some(0);
                        }
                    }
                }
            }
        }
    }
}

// Re-export main API items at the root for convenience
pub use sundo::{
    Actions, 
    HistoryEntry,
    Persistent, 
    PersistentActions, 
    PersistentSnapshot,
    PersistentUndoRedo, 
    PersistentUndoRedoBuilder,
    PersistentScopedTransaction,
    ScopedTransaction,
    Snapshot,
    Tenses,
    UndoRedo, 
    UndoRedoBuilder,
};

#[cfg(test)]
mod test;

