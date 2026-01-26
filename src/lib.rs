//#[allow(dead_code)] //Do not complain about unused structs etc.

pub mod mem_performance_test;

pub mod sundo {
    use std::rc::Rc;

    struct Entry<Payload> {
        payload: Rc<Payload>,
        description: String,
    }

    pub struct UndoRedo<Payload> {
        history: Vec<Entry<Payload>>,
        pointer: usize,
        transaction_start: Option<usize>,
    }

    #[derive(Debug)]
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
    } 

    impl<Payload> Actions<Payload> for UndoRedo<Payload> {
        fn new() -> Self {
            Self {
                history: Vec::with_capacity(100),
                pointer: 0,
                transaction_start: None,
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
        
    }
}

#[cfg(test)]
mod tests {
    use super::sundo::{Actions, UndoRedo, Tenses};

    #[test]
    fn test_new() {
        let undo_redo = UndoRedo::<i32>::new();
        assert_eq!(undo_redo.get_history_len(), 0);
        assert_eq!(undo_redo.get_pointer(), 0);
        assert!(!undo_redo.can_undo());
        assert!(!undo_redo.can_redo());
    }

    #[test]
    fn test_push_single_item() {
        let mut undo_redo = UndoRedo::<i32>::new();
        let rc = undo_redo.push(42, "First item".to_string());
        
        assert_eq!(*rc, 42);
        assert_eq!(undo_redo.get_history_len(), 1);
        assert_eq!(undo_redo.get_pointer(), 0);
        assert!(!undo_redo.can_undo());
        assert!(!undo_redo.can_redo());
    }

    #[test]
    fn test_push_multiple_items() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "First".to_string());
        undo_redo.push(2, "Second".to_string());
        undo_redo.push(3, "Third".to_string());
        
        assert_eq!(undo_redo.get_history_len(), 3);
        assert_eq!(undo_redo.get_pointer(), 2);
        assert!(undo_redo.can_undo());
        assert!(!undo_redo.can_redo());
    }

    #[test]
    fn test_present() {
        let mut undo_redo = UndoRedo::<i32>::new();
        
        // Empty history
        assert!(undo_redo.present().is_none());
        
        // After push
        undo_redo.push(42, "Item".to_string());
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 42);
    }

    #[test]
    fn test_undo_basic() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "First".to_string());
        undo_redo.push(2, "Second".to_string());
        undo_redo.push(3, "Third".to_string());
        
        let result = undo_redo.undo().unwrap();
        assert_eq!(*result, 2);
        assert_eq!(undo_redo.get_pointer(), 1);
        
        let result = undo_redo.undo().unwrap();
        assert_eq!(*result, 1);
        assert_eq!(undo_redo.get_pointer(), 0);
    }

    #[test]
    fn test_undo_at_beginning() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "First".to_string());
        
        assert!(!undo_redo.can_undo());
        assert!(undo_redo.undo().is_none());
    }

    #[test]
    fn test_redo_basic() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "First".to_string());
        undo_redo.push(2, "Second".to_string());
        undo_redo.push(3, "Third".to_string());
        
        undo_redo.undo();
        undo_redo.undo();
        
        let result = undo_redo.redo().unwrap();
        assert_eq!(*result, 2);
        assert_eq!(undo_redo.get_pointer(), 1);
        
        let result = undo_redo.redo().unwrap();
        assert_eq!(*result, 3);
        assert_eq!(undo_redo.get_pointer(), 2);
    }

    #[test]
    fn test_redo_at_end() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "First".to_string());
        undo_redo.push(2, "Second".to_string());
        
        assert!(!undo_redo.can_redo());
        assert!(undo_redo.redo().is_none());
    }

    #[test]
    fn test_push_truncates_future_history() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "First".to_string());
        undo_redo.push(2, "Second".to_string());
        undo_redo.push(3, "Third".to_string());
        
        // Undo twice
        undo_redo.undo();
        undo_redo.undo();
        assert_eq!(undo_redo.get_history_len(), 3);
        
        // Push a new item - should truncate future history
        undo_redo.push(99, "New branch".to_string());
        assert_eq!(undo_redo.get_history_len(), 2);
        assert_eq!(undo_redo.get_pointer(), 1);
        assert!(!undo_redo.can_redo());
        
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 99);
    }

    #[test]
    fn test_update_basic() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(10, "Initial".to_string());
        
        let result = undo_redo.update(|x| x + 5, "Add 5".to_string()).unwrap();
        assert_eq!(*result, 15);
        assert_eq!(undo_redo.get_history_len(), 2);
        
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 15);
    }

    #[test]
    fn test_update_on_empty() {
        let mut undo_redo = UndoRedo::<i32>::new();
        let result = undo_redo.update(|x| x + 1, "Update".to_string());
        assert!(result.is_none());
    }

    #[test]
    fn test_start_transaction_basic() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(0, "Initial".to_string());
        
        undo_redo.start_transaction();
        undo_redo.update(|x| x + 1, "Add 1".to_string());
        undo_redo.update(|x| x + 1, "Add 1 again".to_string());
        undo_redo.update(|x| x + 1, "Add 1 more".to_string());
        
        assert_eq!(undo_redo.get_history_len(), 4);
        
        let success = undo_redo.end_transaction("Add 3".to_string());
        assert!(success);
        
        // Transaction should collapse to single entry
        assert_eq!(undo_redo.get_history_len(), 1);
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 3);
    }

    #[test]
    fn test_end_transaction_without_start() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(5, "Item".to_string());
        
        let success = undo_redo.end_transaction("No transaction".to_string());
        assert!(!success);
    }

    #[test]
    fn test_cancel_transaction() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(10, "Initial".to_string());
        
        undo_redo.start_transaction();
        undo_redo.update(|x| x * 2, "Double".to_string());
        undo_redo.update(|x| x * 2, "Double again".to_string());
        
        assert_eq!(undo_redo.get_history_len(), 3);
        
        let success = undo_redo.cancel_transaction();
        assert!(success);
        
        // Should restore to state before transaction
        assert_eq!(undo_redo.get_history_len(), 1);
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 10);
    }

    #[test]
    fn test_cancel_transaction_without_start() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(5, "Item".to_string());
        
        let success = undo_redo.cancel_transaction();
        assert!(!success);
    }

    #[test]
    fn test_nested_transaction_ignored() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "Initial".to_string());
        
        undo_redo.start_transaction();
        undo_redo.update(|x| x + 1, "Add 1".to_string());
        
        // Second start_transaction should be ignored
        undo_redo.start_transaction();
        undo_redo.update(|x| x + 1, "Add 1 again".to_string());
        
        undo_redo.end_transaction("Transaction".to_string());
        
        // Should still collapse everything from first start
        assert_eq!(undo_redo.get_history_len(), 1);
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 3);
    }

    #[test]
    fn test_get_history_empty() {
        let undo_redo = UndoRedo::<i32>::new();
        assert!(undo_redo.get_history().is_none());
    }

    #[test]
    fn test_get_history_single_item() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(42, "Single item".to_string());
        
        let history = undo_redo.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].description, "Single item");
        assert!(matches!(history[0].tense, Tenses::Present));
        assert_eq!(history[0].pointer, 0);
    }

    #[test]
    fn test_get_history_with_undo() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "First".to_string());
        undo_redo.push(2, "Second".to_string());
        undo_redo.push(3, "Third".to_string());
        
        undo_redo.undo();
        
        let history = undo_redo.get_history().unwrap();
        assert_eq!(history.len(), 3);
        
        assert!(matches!(history[0].tense, Tenses::Past));
        assert!(matches!(history[1].tense, Tenses::Present));
        assert!(matches!(history[2].tense, Tenses::Future));
    }

    #[test]
    fn test_go_to_valid_index() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "First".to_string());
        undo_redo.push(2, "Second".to_string());
        undo_redo.push(3, "Third".to_string());
        undo_redo.push(4, "Fourth".to_string());
        
        let result = undo_redo.go_to(1).unwrap();
        assert_eq!(*result, 2);
        assert_eq!(undo_redo.get_pointer(), 1);
        
        let result = undo_redo.go_to(3).unwrap();
        assert_eq!(*result, 4);
        assert_eq!(undo_redo.get_pointer(), 3);
    }

    #[test]
    fn test_go_to_invalid_index() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "First".to_string());
        undo_redo.push(2, "Second".to_string());
        
        assert!(undo_redo.go_to(5).is_none());
        // Pointer should remain unchanged
        assert_eq!(undo_redo.get_pointer(), 1);
    }

    #[test]
    fn test_go_to_first_item() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "First".to_string());
        undo_redo.push(2, "Second".to_string());
        
        let result = undo_redo.go_to(0).unwrap();
        assert_eq!(*result, 1);
        assert_eq!(undo_redo.get_pointer(), 0);
    }

    #[test]
    fn test_get_pointer() {
        let mut undo_redo = UndoRedo::<i32>::new();
        assert_eq!(undo_redo.get_pointer(), 0);
        
        undo_redo.push(1, "First".to_string());
        assert_eq!(undo_redo.get_pointer(), 0);
        
        undo_redo.push(2, "Second".to_string());
        assert_eq!(undo_redo.get_pointer(), 1);
        
        undo_redo.undo();
        assert_eq!(undo_redo.get_pointer(), 0);
    }

    #[test]
    fn test_get_history_len() {
        let mut undo_redo = UndoRedo::<i32>::new();
        assert_eq!(undo_redo.get_history_len(), 0);
        
        undo_redo.push(1, "First".to_string());
        assert_eq!(undo_redo.get_history_len(), 1);
        
        undo_redo.push(2, "Second".to_string());
        assert_eq!(undo_redo.get_history_len(), 2);
        
        undo_redo.undo();
        assert_eq!(undo_redo.get_history_len(), 2);
        
        undo_redo.push(3, "Third".to_string());
        assert_eq!(undo_redo.get_history_len(), 2);
    }

    #[test]
    fn test_complex_workflow() {
        let mut undo_redo = UndoRedo::<String>::new();
        
        // Build up history
        undo_redo.push("a".to_string(), "Add a".to_string());
        undo_redo.update(|s| format!("{}b", s), "Add b".to_string());
        undo_redo.update(|s| format!("{}c", s), "Add c".to_string());
        
        assert_eq!(*undo_redo.present().unwrap(), "abc");
        
        // Undo back to beginning
        undo_redo.undo();
        undo_redo.undo();
        assert_eq!(*undo_redo.present().unwrap(), "a");
        
        // Branch to new history
        undo_redo.update(|s| format!("{}x", s), "Add x".to_string());
        assert_eq!(undo_redo.get_history_len(), 2);
        assert_eq!(*undo_redo.present().unwrap(), "ax");
        
        // Can't redo anymore
        assert!(!undo_redo.can_redo());
    }

    #[test]
    fn test_transaction_with_no_changes() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(5, "Initial".to_string());
        
        undo_redo.start_transaction();
        // Don't make any changes
        let success = undo_redo.end_transaction("Empty transaction".to_string());
        
        assert!(success);
        assert_eq!(undo_redo.get_history_len(), 1);
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 5);
    }

    #[test]
    fn test_undo_after_transaction() {
        let mut undo_redo = UndoRedo::<i32>::new();
        undo_redo.push(1, "Initial".to_string());
        
        undo_redo.start_transaction();
        undo_redo.update(|x| x + 1, "Inc".to_string());
        undo_redo.update(|x| x + 1, "Inc".to_string());
        undo_redo.end_transaction("Add 2".to_string());
        
        assert_eq!(undo_redo.get_history_len(), 1);
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 3);
        
        // Can't undo because transaction collapsed to the starting point
        assert!(!undo_redo.can_undo());
    }

    #[test]
    fn test_rc_sharing() {
        let mut undo_redo = UndoRedo::<Vec<i32>>::new();
        let rc1 = undo_redo.push(vec![1, 2, 3], "Initial".to_string());
        
        // Get another reference via present
        let rc2 = undo_redo.present().unwrap();
        
        // Both should point to the same data
        assert_eq!(*rc1, *rc2);
        assert_eq!(rc1.as_ptr(), rc2.as_ptr());
    }
}
