//#[allow(dead_code)] //Do not complain about unused structs etc.

pub mod undo_redo {
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
            self.pointer < self.history.len() - 1
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
        
    }
}
