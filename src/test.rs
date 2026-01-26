#[cfg(test)]
mod tests {
    use crate::sundo::{Actions, UndoRedo, Tenses, Persistent, PersistentActions, PersistentUndoRedo};

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
        
        // Transaction should collapse to 2 entries: initial + transaction result
        assert_eq!(undo_redo.get_history_len(), 2);
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
        
        // Should collapse to 2 entries: initial + transaction result
        assert_eq!(undo_redo.get_history_len(), 2);
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
        
        assert_eq!(undo_redo.get_history_len(), 2);
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 3);
        
        // Should be able to undo to state before transaction
        assert!(undo_redo.can_undo());
        undo_redo.undo();
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 1);
    }

    #[test]
    fn test_transaction_does_not_overwrite_previous_state() {
        let mut undo_redo = UndoRedo::<i32>::new();
        
        // Step 1: Add initial state
        undo_redo.push(0, "Initial".to_string());
        
        // Step 2: Add an item (state before transaction)
        undo_redo.update(|x| x + 10, "Add 10".to_string());
        assert_eq!(*undo_redo.present().unwrap(), 10);
        
        // Step 3: Start transaction
        undo_redo.start_transaction();
        
        // Step 4: Add another item inside transaction
        undo_redo.update(|x| x + 5, "Add 5".to_string());
        
        // Step 5: End transaction
        undo_redo.end_transaction("Transaction: Add 5".to_string());
        
        // Should have 3 states: initial, +10, transaction
        assert_eq!(undo_redo.get_history_len(), 3);
        assert_eq!(*undo_redo.present().unwrap(), 15);
        
        // Step 6: Undo - should go back to state before transaction (10)
        undo_redo.undo();
        let present = undo_redo.present().unwrap();
        assert_eq!(*present, 10, "First added item should still exist after undo");
        
        // Should still be able to undo to initial state
        assert!(undo_redo.can_undo());
        undo_redo.undo();
        assert_eq!(*undo_redo.present().unwrap(), 0);
    }

    #[test]
    fn test_persistent_transaction_does_not_overwrite_previous_state() {
        use im::Vector;
        
        let mut undo_redo = PersistentUndoRedo::new();
        
        // Step 1: Add initial state
        undo_redo.push(Vector::new(), "Initial".to_string());
        
        // Step 2: Add an item (state before transaction)
        undo_redo.update(|v| {
            let mut new_v = v.clone();
            new_v.push_back(1);
            new_v
        }, "Add 1".to_string());
        assert_eq!(undo_redo.present().unwrap().len(), 1);
        
        // Step 3: Start transaction
        undo_redo.start_transaction();
        
        // Step 4: Add items inside transaction
        undo_redo.update(|v| {
            let mut new_v = v.clone();
            new_v.push_back(2);
            new_v
        }, "Add 2".to_string());
        undo_redo.update(|v| {
            let mut new_v = v.clone();
            new_v.push_back(3);
            new_v
        }, "Add 3".to_string());
        
        // Step 5: End transaction
        undo_redo.end_transaction("Transaction: Add 2,3".to_string());
        
        // Should have 3 states: initial, +1, transaction
        assert_eq!(undo_redo.get_history_len(), 3);
        assert_eq!(undo_redo.present().unwrap().len(), 3);
        
        // Step 6: Undo - should go back to state before transaction ([1])
        undo_redo.undo();
        let present = undo_redo.present().unwrap();
        assert_eq!(present.len(), 1, "First added item should still exist after undo");
        assert_eq!(present[0], 1);
        
        // Should still be able to undo to initial state
        assert!(undo_redo.can_undo());
        undo_redo.undo();
        assert_eq!(undo_redo.present().unwrap().len(), 0);
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
    
    #[test]
    fn test_persistent_trait_with_im_vector() {
        use im::Vector;
        
        // im::Vector implements Persistent
        fn assert_persistent<T: Persistent>(_: &T) {}
        
        let vec = Vector::from(vec![1, 2, 3]);
        assert_persistent(&vec);
        
        let mut undo_redo = UndoRedo::new();
        undo_redo.push(vec.clone(), "Initial".to_string());
        
        let mut vec2 = vec.clone();
        vec2.push_back(4);
        undo_redo.push(vec2, "Add 4".to_string());
        
        assert_eq!(undo_redo.get_history_len(), 2);
        
        let current = undo_redo.present().unwrap();
        assert_eq!(current.len(), 4);
        
        undo_redo.undo();
        let current = undo_redo.present().unwrap();
        assert_eq!(current.len(), 3);
    }
    
    #[test]
    fn test_persistent_trait_with_im_hashmap() {
        use im::HashMap;
        
        // im::HashMap implements Persistent
        fn assert_persistent<T: Persistent>(_: &T) {}
        
        let map = HashMap::new();
        assert_persistent(&map);
        
        let mut undo_redo = UndoRedo::new();
        let map = map.update("key1", "value1");
        undo_redo.push(map.clone(), "Add key1".to_string());
        
        let map = map.update("key2", "value2");
        undo_redo.push(map, "Add key2".to_string());
        
        let current = undo_redo.present().unwrap();
        assert_eq!(current.len(), 2);
        
        undo_redo.undo();
        let current = undo_redo.present().unwrap();
        assert_eq!(current.len(), 1);
    }
    
    #[test]
    fn test_custom_persistent_type() {
        use im::{Vector, HashMap};
        
        #[derive(Clone)]
        struct AppState {
            users: HashMap<u64, String>,
            messages: Vector<String>,
        }
        
        impl Persistent for AppState {}
        
        // Should compile - AppState implements Persistent
        fn assert_persistent<T: Persistent>(_: &T) {}
        
        let state = AppState {
            users: HashMap::new(),
            messages: Vector::new(),
        };
        assert_persistent(&state);
        
        let mut undo_redo = UndoRedo::new();
        undo_redo.push(state, "Initial".to_string());
        
        assert_eq!(undo_redo.get_history_len(), 1);
    }
    
    // ============================================================================
    // Tests for PersistentUndoRedo (optimized storage)
    // ============================================================================
    
    #[test]
    fn test_persistent_undo_redo_basic() {
        use im::Vector;
        
        let mut undo_redo = PersistentUndoRedo::new();
        let vec = Vector::from(vec![1, 2, 3]);
        
        // Push returns &Payload, not Rc<Payload>
        let pushed = undo_redo.push(vec, "Initial".to_string());
        assert_eq!(pushed.len(), 3);
        
        // Present returns Option<&Payload>
        let current = undo_redo.present().unwrap();
        assert_eq!(current.len(), 3);
        assert_eq!(*current, Vector::from(vec![1, 2, 3]));
    }
    
    #[test]
    fn test_persistent_undo_redo_update() {
        use im::Vector;
        
        let mut undo_redo = PersistentUndoRedo::new();
        let vec = Vector::from(vec![1, 2, 3]);
        undo_redo.push(vec, "Initial".to_string());
        
        // Update creates new version via structural sharing
        undo_redo.update(
            |v| {
                let mut new_v = v.clone();
                new_v.push_back(4);
                new_v
            },
            "Add 4".to_string()
        );
        
        let current = undo_redo.present().unwrap();
        assert_eq!(current.len(), 4);
        assert_eq!(*current, Vector::from(vec![1, 2, 3, 4]));
    }
    
    #[test]
    fn test_persistent_undo_redo_undo_redo() {
        use im::HashMap;
        
        let mut undo_redo = PersistentUndoRedo::new();
        let map = HashMap::new();
        undo_redo.push(map, "Empty".to_string());
        
        undo_redo.update(|m| m.update("a", 1), "Add a".to_string());
        undo_redo.update(|m| m.update("b", 2), "Add b".to_string());
        undo_redo.update(|m| m.update("c", 3), "Add c".to_string());
        
        assert_eq!(undo_redo.present().unwrap().len(), 3);
        
        // Undo twice
        undo_redo.undo();
        assert_eq!(undo_redo.present().unwrap().len(), 2);
        
        undo_redo.undo();
        assert_eq!(undo_redo.present().unwrap().len(), 1);
        
        // Redo once
        undo_redo.redo();
        assert_eq!(undo_redo.present().unwrap().len(), 2);
    }
    
    #[test]
    fn test_persistent_transaction() {
        use im::Vector;
        
        let mut undo_redo = PersistentUndoRedo::new();
        let vec = Vector::from(vec![1]);
        undo_redo.push(vec, "Initial".to_string());
        
        undo_redo.start_transaction();
        undo_redo.update(|v| {
            let mut new_v = v.clone();
            new_v.push_back(2);
            new_v
        }, "Add 2".to_string());
        undo_redo.update(|v| {
            let mut new_v = v.clone();
            new_v.push_back(3);
            new_v
        }, "Add 3".to_string());
        undo_redo.end_transaction("Batch add".to_string());
        
        assert_eq!(undo_redo.get_history_len(), 2);
        let current = undo_redo.present().unwrap();
        assert_eq!(current.len(), 3);
    }
    
    #[test]
    fn test_persistent_cancel_transaction() {
        use im::Vector;
        
        let mut undo_redo = PersistentUndoRedo::new();
        let vec = Vector::from(vec![1]);
        undo_redo.push(vec, "Initial".to_string());
        
        undo_redo.start_transaction();
        undo_redo.update(|v| {
            let mut new_v = v.clone();
            new_v.push_back(2);
            new_v
        }, "Add 2".to_string());
        undo_redo.cancel_transaction();
        
        // Should be back to initial state
        let current = undo_redo.present().unwrap();
        assert_eq!(current.len(), 1);
        assert_eq!(undo_redo.get_history_len(), 1);
    }
    
    #[test]
    fn test_persistent_custom_type() {
        use im::{Vector, HashMap};
        
        #[derive(Clone, PartialEq, Debug)]
        struct AppState {
            users: HashMap<u64, String>,
            messages: Vector<String>,
        }
        
        impl Persistent for AppState {}
        
        impl AppState {
            fn add_user(&self, id: u64, name: String) -> Self {
                Self {
                    users: self.users.update(id, name),
                    messages: self.messages.clone(),
                }
            }
        }
        
        let mut undo_redo = PersistentUndoRedo::new();
        let state = AppState {
            users: HashMap::new(),
            messages: Vector::new(),
        };
        undo_redo.push(state, "Initial".to_string());
        
        undo_redo.update(
            |s| s.add_user(1, "Alice".to_string()),
            "Add Alice".to_string()
        );
        
        let current = undo_redo.present().unwrap();
        assert_eq!(current.users.len(), 1);
        assert_eq!(current.users.get(&1), Some(&"Alice".to_string()));
    }
    
    #[test]
    fn test_persistent_go_to() {
        use im::Vector;
        
        let mut undo_redo = PersistentUndoRedo::new();
        
        for i in 0..5 {
            let mut vec = Vector::new();
            for j in 0..=i {
                vec.push_back(j);
            }
            undo_redo.push(vec, format!("State {}", i));
        }
        
        assert_eq!(undo_redo.get_history_len(), 5);
        
        // Go to middle
        let state = undo_redo.go_to(2).unwrap();
        assert_eq!(state.len(), 3);
        assert_eq!(undo_redo.get_pointer(), 2);
        
        // Go to start
        let state = undo_redo.go_to(0).unwrap();
        assert_eq!(state.len(), 1);
    }
    
    #[test]
    fn test_persistent_get_history() {
        use im::Vector;
        
        let mut undo_redo = PersistentUndoRedo::new();
        
        undo_redo.push(Vector::from(vec![1]), "One".to_string());
        undo_redo.update(|v| {
            let mut new_v = v.clone();
            new_v.push_back(2);
            new_v
        }, "Two".to_string());
        undo_redo.update(|v| {
            let mut new_v = v.clone();
            new_v.push_back(3);
            new_v
        }, "Three".to_string());
        
        let history = undo_redo.get_history().unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].description, "One");
        assert_eq!(history[1].description, "Two");
        assert_eq!(history[2].description, "Three");
        assert_eq!(history[2].tense, Tenses::Present);
        
        undo_redo.undo();
        let history = undo_redo.get_history().unwrap();
        assert_eq!(history[1].tense, Tenses::Present);
        assert_eq!(history[2].tense, Tenses::Future);
    }
    
    #[test]
    fn test_persistent_can_undo_redo() {
        use im::Vector;
        
        let mut undo_redo = PersistentUndoRedo::new();
        
        // Empty history
        assert!(!undo_redo.can_undo());
        assert!(!undo_redo.can_redo());
        
        undo_redo.push(Vector::from(vec![1]), "One".to_string());
        assert!(!undo_redo.can_undo());
        assert!(!undo_redo.can_redo());
        
        undo_redo.update(|v| {
            let mut new_v = v.clone();
            new_v.push_back(2);
            new_v
        }, "Two".to_string());
        assert!(undo_redo.can_undo());
        assert!(!undo_redo.can_redo());
        
        undo_redo.undo();
        assert!(!undo_redo.can_undo());
        assert!(undo_redo.can_redo());
    }
    
    #[test]
    fn test_persistent_no_rc_overhead() {
        use im::Vector;
        
        // Verify PersistentEntry doesn't have Rc overhead
        let mut undo_redo = PersistentUndoRedo::new();
        let vec = Vector::from(vec![1, 2, 3]);
        let pushed = undo_redo.push(vec, "Test".to_string());
        
        // With PersistentUndoRedo, we get direct reference
        // Not wrapped in Rc like standard UndoRedo
        assert_eq!(pushed.len(), 3);
        
        // The key benefit: PersistentUndoRedo stores payloads directly
        // without Rc wrapping, avoiding one level of indirection and
        // the associated heap allocation overhead.
        // This test verifies the API works correctly with direct references.
        let current = undo_redo.present().unwrap();
        assert_eq!(current.len(), 3);
        
        // Multiple entries share structure internally via im::Vector
        undo_redo.update(|v| {
            let mut new_v = v.clone();
            new_v.push_back(4);
            new_v
        }, "Add 4".to_string());
        
        assert_eq!(undo_redo.present().unwrap().len(), 4);
    }
}

// ============================================================================
// SCOPED TRANSACTION TESTS
// ============================================================================

#[cfg(test)]
mod scoped_transaction_tests {
    use crate::sundo::{Actions, PersistentActions, PersistentScopedTransaction, PersistentUndoRedo, ScopedTransaction, UndoRedo};
    use im::Vector;

    #[test]
    fn test_scoped_transaction_commit() {
        let mut undo_redo = UndoRedo::new();
        undo_redo.push(0, "Initial".to_string());
        
        {
            let mut tx = ScopedTransaction::begin(&mut undo_redo, "Add 10");
            tx.get().update(|x| x + 3, "temp".to_string());
            tx.get().update(|x| x + 3, "temp".to_string());
            tx.get().update(|x| x + 4, "temp".to_string());
            tx.commit();
        }
        
        assert_eq!(*undo_redo.present().unwrap(), 10);
        assert_eq!(undo_redo.get_history_len(), 2);  // Initial + transaction result
    }

    #[test]
    fn test_scoped_transaction_rollback() {
        let mut undo_redo = UndoRedo::new();
        undo_redo.push(5, "Initial".to_string());
        
        let before = *undo_redo.present().unwrap();
        
        {
            let mut tx = ScopedTransaction::begin(&mut undo_redo, "This will rollback");
            tx.get().update(|x| x + 100, "temp".to_string());
            tx.get().update(|x| x + 200, "temp".to_string());
            // Don't commit - should auto-rollback
        }
        
        assert_eq!(*undo_redo.present().unwrap(), before);
        assert_eq!(undo_redo.get_history_len(), 1);  // Still just initial state
    }

    #[test]
    fn test_persistent_scoped_transaction_commit() {
        let mut undo_redo = PersistentUndoRedo::new();
        undo_redo.push(Vector::<i32>::new(), "Initial".to_string());
        
        {
            let mut tx = PersistentScopedTransaction::begin(&mut undo_redo, "Add items");
            tx.get().update(|v: &Vector<i32>| { 
                let mut new_v = v.clone(); 
                new_v.push_back(1); 
                new_v 
            }, "temp".to_string());
            tx.get().update(|v: &Vector<i32>| { 
                let mut new_v = v.clone(); 
                new_v.push_back(2); 
                new_v 
            }, "temp".to_string());
            tx.get().update(|v: &Vector<i32>| { 
                let mut new_v = v.clone(); 
                new_v.push_back(3); 
                new_v 
            }, "temp".to_string());
            tx.commit();
        }
        
        let result = undo_redo.present().unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(undo_redo.get_history_len(), 2);
    }

    #[test]
    fn test_persistent_scoped_transaction_rollback() {
        let mut undo_redo = PersistentUndoRedo::new();
        let initial = Vector::from(vec![1, 2, 3]);
        undo_redo.push(initial.clone(), "Initial".to_string());
        
        {
            let mut tx = PersistentScopedTransaction::begin(&mut undo_redo, "This will rollback");
            tx.get().update(|v: &Vector<i32>| { 
                let mut new_v = v.clone(); 
                new_v.push_back(99); 
                new_v 
            }, "temp".to_string());
            tx.get().update(|v: &Vector<i32>| { 
                let mut new_v = v.clone(); 
                new_v.push_back(100); 
                new_v 
            }, "temp".to_string());
            // Don't commit
        }
        
        assert_eq!(undo_redo.present().unwrap(), &initial);
        assert_eq!(undo_redo.get_history_len(), 1);
    }
}

// ============================================================================
// BUILDER PATTERN AND MAX ENTRIES TESTS
// ============================================================================

#[cfg(test)]
mod builder_tests {
    use crate::sundo::{Actions, PersistentActions, PersistentUndoRedoBuilder, UndoRedoBuilder};
    use im::Vector;

    #[test]
    fn test_builder_default() {
        let mut undo_redo = UndoRedoBuilder::<i32>::new().build();
        undo_redo.push(42, "First".to_string());
        assert_eq!(*undo_redo.present().unwrap(), 42);
    }

    #[test]
    fn test_builder_with_capacity() {
        let undo_redo = UndoRedoBuilder::<String>::new()
            .with_capacity(1000)
            .build();
        assert_eq!(undo_redo.get_history_len(), 0);
    }

    #[test]
    fn test_max_entries_pruning() {
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_entries(5)
            .build();
        
        // Add 10 entries
        for i in 0..10 {
            undo_redo.push(i, format!("Step {}", i));
        }
        
        // Should only keep the last 5
        assert_eq!(undo_redo.get_history_len(), 5);
        assert_eq!(*undo_redo.present().unwrap(), 9);
    }

    #[test]
    fn test_max_entries_preserves_current_state() {
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_entries(3)
            .build();
        
        // Add entries
        for i in 0..10 {
            undo_redo.push(i, format!("Value {}", i));
        }
        
        // Current state should be preserved
        assert_eq!(*undo_redo.present().unwrap(), 9);
        assert_eq!(undo_redo.get_history_len(), 3);
    }

    #[test]
    fn test_max_entries_with_undo() {
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_entries(5)
            .build();
        
        // Add 10 entries
        for i in 0..10 {
            undo_redo.push(i, format!("Step {}", i));
        }
        
        // Should be able to undo within the kept history
        assert_eq!(undo_redo.get_history_len(), 5);
        
        // Undo a few times
        undo_redo.undo();
        assert_eq!(*undo_redo.present().unwrap(), 8);
        
        undo_redo.undo();
        assert_eq!(*undo_redo.present().unwrap(), 7);
    }

    #[test]
    fn test_max_entries_with_update() {
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_entries(3)
            .build();
        
        undo_redo.push(1, "Initial".to_string());
        
        for i in 0..10 {
            undo_redo.update(|x| x + 1, format!("Increment {}", i));
        }
        
        assert_eq!(undo_redo.get_history_len(), 3);
        assert_eq!(*undo_redo.present().unwrap(), 11);
    }

    #[test]
    fn test_persistent_builder_max_entries() {
        let mut undo_redo = PersistentUndoRedoBuilder::new()
            .with_max_entries(5)
            .build();
        
        // Add 10 Vector states
        for i in 0..10 {
            let vec = Vector::from(vec![i]);
            undo_redo.push(vec, format!("Step {}", i));
        }
        
        assert_eq!(undo_redo.get_history_len(), 5);
        assert_eq!(undo_redo.present().unwrap().len(), 1);
        assert_eq!(undo_redo.present().unwrap()[0], 9);
    }

    #[test]
    fn test_no_limit_unlimited_history() {
        let mut undo_redo = UndoRedoBuilder::new().build();
        
        // Add many entries
        for i in 0..100 {
            undo_redo.push(i, format!("Step {}", i));
        }
        
        // All should be preserved
        assert_eq!(undo_redo.get_history_len(), 100);
    }

    #[test]
    fn test_max_entries_one() {
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_entries(1)
            .build();
        
        undo_redo.push(1, "First".to_string());
        undo_redo.push(2, "Second".to_string());
        undo_redo.push(3, "Third".to_string());
        
        // Should only keep current state
        assert_eq!(undo_redo.get_history_len(), 1);
        assert_eq!(*undo_redo.present().unwrap(), 3);
        
        // Can't undo since we only have current state
        assert!(!undo_redo.can_undo());
    }

    #[test]
    fn test_max_entries_pointer_adjustment() {
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_entries(5)
            .build();
        
        // Add entries
        for i in 0..10 {
            undo_redo.push(i, format!("Step {}", i));
        }
        
        // Pointer should be at the end
        assert_eq!(undo_redo.get_pointer(), undo_redo.get_history_len() - 1);
    }

    // ============================================================================
    // Memory-Based Limits Tests
    // ============================================================================

    #[test]
    fn test_memory_footprint_for_string() {
        use crate::sundo::MemoryFootprint;
        
        let s = String::from("hello");
        let footprint = s.memory_footprint();
        
        // Should include base size + capacity
        assert!(footprint >= std::mem::size_of::<String>() + s.capacity());
    }

    #[test]
    fn test_memory_footprint_for_vec() {
        use crate::sundo::MemoryFootprint;
        
        let v = vec![1, 2, 3, 4, 5];
        let footprint = v.memory_footprint();
        
        // Should include base size + elements
        assert!(footprint >= std::mem::size_of::<Vec<i32>>() + 5 * std::mem::size_of::<i32>());
    }

    #[test]
    fn test_memory_footprint_for_im_vector() {
        use crate::sundo::MemoryFootprint;
        use im::Vector;
        
        let v: Vector<i32> = Vector::from(vec![1, 2, 3, 4, 5]);
        let footprint = v.memory_footprint();
        
        // Should have some footprint
        assert!(footprint > 0);
    }

    #[test]
    fn test_memory_based_limit_basic() {
        use crate::sundo::UndoRedoBuilder;
        
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_memory_bytes(1000)
            .build();
        
        // Add a string
        undo_redo.push("Small".to_string(), "Initial".to_string());
        
        // Add more strings
        for i in 0..10 {
            undo_redo.update(
                |s| format!("{} {}", s, "More text to increase memory"),
                format!("Update {}", i)
            );
            
            // Manually call prune to test memory-based pruning
            undo_redo.prune();
        }
        
        // History should be limited by memory
        assert!(undo_redo.get_history_len() > 0);
        assert!(undo_redo.get_history_len() < 11); // Should be less than total pushes
    }

    #[test]
    fn test_memory_based_limit_with_vec() {
        use crate::sundo::UndoRedoBuilder;
        
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_memory_bytes(5000)
            .build();
        
        undo_redo.push(Vec::<i32>::new(), "Empty".to_string());
        
        // Add many elements
        for i in 0..100 {
            undo_redo.update(
                |v| {
                    let mut new_v = v.clone();
                    new_v.push(i);
                    new_v
                },
                format!("Add {}", i)
            );
            
            if i % 10 == 0 {
                undo_redo.prune();
            }
        }
        
        // Should have been pruned
        assert!(undo_redo.get_history_len() < 100);
    }

    #[test]
    fn test_combined_limits() {
        use crate::sundo::UndoRedoBuilder;
        
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_entries(20)
            .with_max_memory_bytes(10000)
            .build();
        
        undo_redo.push("test".to_string(), "Initial".to_string());
        
        for i in 0..50 {
            undo_redo.update(
                |s| format!("{} {}", s, i),
                format!("Update {}", i)
            );
            undo_redo.prune();
        }
        
        // Both limits should be respected
        assert!(undo_redo.get_history_len() <= 20);
    }

    #[test]
    fn test_memory_limit_preserves_current_state() {
        use crate::sundo::UndoRedoBuilder;
        
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_memory_bytes(500)
            .build();
        
        undo_redo.push(String::from("initial"), "Initial".to_string());
        
        let mut last_value = String::from("initial");
        for i in 0..20 {
            last_value = format!("{} more_text_{}", last_value, i);
            undo_redo.update(
                |_| last_value.clone(),
                format!("Update {}", i)
            );
            undo_redo.prune();
        }
        
        // Current state should always be preserved
        assert_eq!(*undo_redo.present().unwrap(), last_value);
    }

    #[test]
    fn test_persistent_with_memory_limits() {
        use crate::sundo::PersistentUndoRedoBuilder;
        use im::Vector;
        
        let mut undo_redo = PersistentUndoRedoBuilder::new()
            .with_max_memory_mb(1)
            .build();
        
        undo_redo.push(Vector::<i32>::new(), "Empty".to_string());
        
        for i in 0..100 {
            undo_redo.update(
                |v| {
                    let mut new_v = v.clone();
                    new_v.push_back(i);
                    new_v
                },
                format!("Add {}", i)
            );
            
            if i % 10 == 0 {
                undo_redo.prune();
            }
        }
        
        // Should have been pruned but current state preserved
        assert!(undo_redo.get_history_len() > 0);
        assert_eq!(undo_redo.present().unwrap().len(), 100);
    }

    #[test]
    fn test_snapshot_with_memory_info() {
        use crate::sundo::UndoRedoBuilder;
        
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_entries(10)
            .with_max_memory_bytes(5000)
            .build();
        
        undo_redo.push(String::from("test"), "Initial".to_string());
        
        let snapshot = undo_redo.export_snapshot();
        
        assert_eq!(snapshot.max_entries, Some(10));
        assert_eq!(snapshot.max_memory_bytes, Some(5000));
        assert!(snapshot.current_memory_bytes >= 0);
    }

    #[test]
    fn test_restore_from_snapshot_with_memory_limits() {
        use crate::sundo::{UndoRedoBuilder, UndoRedo};
        
        let mut undo_redo = UndoRedoBuilder::new()
            .with_max_memory_mb(1)
            .build();
        
        undo_redo.push(42, "Initial".to_string());
        undo_redo.update(|x| x + 10, "Add 10".to_string());
        
        let snapshot = undo_redo.export_snapshot();
        
        // Restore from snapshot
        let restored: UndoRedo<i32> = UndoRedo::from_snapshot(snapshot);
        
        assert_eq!(*restored.present().unwrap(), 52);
        assert_eq!(restored.get_history_len(), 2);
    }
}

