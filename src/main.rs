use undo_redo::sundo::{Actions, UndoRedo};

#[derive(Clone, Debug)]
struct Todo {
    description: String,
    done: bool,
}

#[derive(Clone, Debug)]
struct Model {
    todos: Vec<Todo>,
}

trait TodoOperations {
    fn add_todo(&self, description: String) -> Self;
    fn toggle_todo(&self, index: usize) -> Option<Self>
    where
        Self: Sized;
    fn remove_todo(&self, index: usize) -> Option<Self>
    where
        Self: Sized;
}

impl TodoOperations for Model {
    fn add_todo(&self, description: String) -> Self {
        let mut new_todos = self.todos.clone();
        new_todos.push(Todo {
            description,
            done: false,
        });
        Model { todos: new_todos }
    }

    fn toggle_todo(&self, index: usize) -> Option<Self> {
        if index >= self.todos.len() {
            return None;
        }
        let mut new_todos = self.todos.clone();
        new_todos[index].done = !new_todos[index].done;
        Some(Model { todos: new_todos })
    }

    fn remove_todo(&self, index: usize) -> Option<Self> {
        if index >= self.todos.len() {
            return None;
        }
        let mut new_todos = self.todos.clone();
        new_todos.remove(index);
        Some(Model { todos: new_todos })
    }
}

fn main() {
    let mut undo_redo = UndoRedo::<Model>::new();

    // Start with empty model
    undo_redo.push(Model { todos: vec![] }, "Initial state".to_string());

    // Add todos using update()
    for i in 0..5 {
        undo_redo.update(
            |model| model.add_todo(format!("Todo {}", i)),
            format!("Added todo {}", i)
        );
    }

    undo_redo.undo();
    undo_redo.undo();

    // Print history
    if let Some(history) = undo_redo.get_history() {
        println!("\nHistory:");
        for entry in history {
            println!(
                "  [{}] {}: {:?}",
                entry.pointer, entry.description, entry.tense
            );
        }
    }

    undo_redo.update(
        |model| model.add_todo(format!("Todo {}", 5)),
        format!("Added todo {}", 5)
    );

    if let Some(history) = undo_redo.get_history() {
        println!("\nHistory:");
        for entry in history {
            println!(
                "  [{}] {}: {:?}",
                entry.pointer, entry.description, entry.tense
            );
        }
    }

    // Demonstrate transactions - add multiple todos as one undoable action
    println!("\n=== Testing Transactions ===");
    
    undo_redo.start_transaction();
    undo_redo.update(|model| model.add_todo("Transaction Todo 1".to_string()), "Temp 1".to_string());
    undo_redo.update(|model| model.add_todo("Transaction Todo 2".to_string()), "Temp 2".to_string());
    undo_redo.update(|model| model.add_todo("Transaction Todo 3".to_string()), "Temp 3".to_string());
    undo_redo.end_transaction("Added 3 todos in one transaction".to_string());

    if let Some(history) = undo_redo.get_history() {
        println!("\nHistory after transaction:");
        for entry in history {
            println!(
                "  [{}] {}: {:?}",
                entry.pointer, entry.description, entry.tense
            );
        }
    }

    // Undo the entire transaction
    undo_redo.undo();
    
    if let Some(history) = undo_redo.get_history() {
        println!("\nHistory after undoing transaction:");
        for entry in history {
            println!(
                "  [{}] {}: {:?}",
                entry.pointer, entry.description, entry.tense
            );
        }
    }

    // Test cancel_transaction
    println!("\n=== Testing Cancel Transaction ===");
    undo_redo.start_transaction();
    undo_redo.update(|model| model.add_todo("Should be cancelled 1".to_string()), "Cancelled 1".to_string());
    undo_redo.update(|model| model.add_todo("Should be cancelled 2".to_string()), "Cancelled 2".to_string());
    undo_redo.cancel_transaction();

    if let Some(history) = undo_redo.get_history() {
        println!("\nHistory after cancelling transaction:");
        for entry in history {
            println!(
                "  [{}] {}: {:?}",
                entry.pointer, entry.description, entry.tense
            );
        }
    }


}
