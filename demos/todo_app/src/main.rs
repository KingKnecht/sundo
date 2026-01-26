use sundo::{PersistentActions, PersistentUndoRedo, Persistent};
use im::Vector;
use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    DefaultTerminal, Frame,
};

#[derive(Clone, Debug)]
struct Todo {
    id: u32,
    text: String,
    completed: bool,
}

#[derive(Clone, Debug)]
struct TodoState {
    todos: Vector<Todo>,
    next_id: u32,
}

impl Persistent for TodoState {}

impl TodoState {
    fn new() -> Self {
        Self {
            todos: Vector::new(),
            next_id: 1,
        }
    }
    
    fn add_todo(&self, text: String) -> Self {
        let todo = Todo {
            id: self.next_id,
            text,
            completed: false,
        };
        let mut new_todos = self.todos.clone();
        new_todos.push_back(todo);
        Self {
            todos: new_todos,
            next_id: self.next_id + 1,
        }
    }
    
    fn toggle_todo(&self, id: u32) -> Self {
        let new_todos: Vector<Todo> = self.todos
            .iter()
            .map(|todo| {
                if todo.id == id {
                    let mut updated = todo.clone();
                    updated.completed = !updated.completed;
                    updated
                } else {
                    todo.clone()
                }
            })
            .collect();
        Self {
            todos: new_todos,
            next_id: self.next_id,
        }
    }
    
    fn delete_todo(&self, id: u32) -> Self {
        let new_todos: Vector<Todo> = self.todos
            .iter()
            .filter(|t| t.id != id)
            .cloned()
            .collect();
        Self {
            todos: new_todos,
            next_id: self.next_id,
        }
    }
}

struct App {
    undo_redo: PersistentUndoRedo<TodoState>,
    input: String,
    selected_todo: Option<u32>,
    in_transaction: bool,
}

impl Default for App {
    fn default() -> Self {
        let mut undo_redo = PersistentUndoRedo::new();
        undo_redo.push(TodoState::new(), "Init".to_string());
        Self {
            undo_redo,
            input: String::new(),
            selected_todo: None,
            in_transaction: false,
        }
    }
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;
            
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('u') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            self.undo_redo.undo();
                        }
                        KeyCode::Char('r') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            self.undo_redo.redo();
                        }
                        KeyCode::Char('t') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            if self.in_transaction {
                                self.undo_redo.end_transaction("Transaction".to_string());
                                self.in_transaction = false;
                            } else {
                                self.undo_redo.start_transaction();
                                self.in_transaction = true;
                            }
                        }
                        KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) && self.in_transaction => {
                            self.undo_redo.cancel_transaction();
                            self.in_transaction = false;
                        }
                        KeyCode::Enter => {
                            if !self.input.is_empty() {
                                let text = self.input.clone();
                                self.undo_redo.update(|state| state.add_todo(text), "Add todo".to_string());
                                self.input.clear();
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some(id) = self.selected_todo {
                                self.undo_redo.update(|state| state.toggle_todo(id), "Toggle todo".to_string());
                            } else {
                                self.input.push(' ');
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(id) = self.selected_todo {
                                self.undo_redo.update(|state| state.delete_todo(id), "Delete todo".to_string());
                                self.selected_todo = None;
                            } else {
                                self.input.push('d');
                            }
                        }
                        KeyCode::Char(c) => {
                            self.input.push(c);
                        }
                        KeyCode::Backspace => {
                            self.input.pop();
                        }
                        KeyCode::Down => {
                            if let Some(state) = self.undo_redo.present() {
                                if !state.todos.is_empty() {
                                    self.selected_todo = Some(
                                        self.selected_todo
                                            .and_then(|id| {
                                                let pos = state.todos.iter().position(|t| t.id == id)?;
                                                state.todos.get(pos + 1).map(|t| t.id)
                                            })
                                            .unwrap_or_else(|| state.todos[0].id)
                                    );
                                }
                            }
                        }
                        KeyCode::Up => {
                            if let Some(state) = self.undo_redo.present() {
                                if !state.todos.is_empty() {
                                    self.selected_todo = Some(
                                        self.selected_todo
                                            .and_then(|id| {
                                                let pos = state.todos.iter().position(|t| t.id == id)?;
                                                if pos > 0 {
                                                    state.todos.get(pos - 1).map(|t| t.id)
                                                } else {
                                                    Some(id)
                                                }
                                            })
                                            .unwrap_or_else(|| state.todos[0].id)
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);
        
        let top_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Percentage(30),
            ])
            .split(main_layout[1]);
        
        // Title
        let title = Paragraph::new("Ratatui To-Do App with Undo/Redo powered by Sundo")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(title, main_layout[0]);
        
        // Left side - Todo list
        let left_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(top_layout[0]);
        
        // Input box
        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default()
                .borders(Borders::ALL)
                .title("New Todo (Enter to add)"));
        frame.render_widget(input, left_layout[0]);
        
        // Todo list
        let todos: Vec<ListItem> = self.undo_redo.present()
            .map(|state| state.todos
            .iter()
            .map(|todo| {
                let checkbox = if todo.completed { "[✓]" } else { "[ ]" };
                let style = if Some(todo.id) == self.selected_todo {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default()
                };
                let content = format!("{} {}", checkbox, todo.text);
                ListItem::new(content).style(style)
            })
            .collect())
            .unwrap_or_else(Vec::new);
        
        let list = List::new(todos)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Todos (↑/↓: select, Space: toggle, d: delete)"));
        frame.render_widget(list, left_layout[1]);
        
        // Right side - Controls and Stack
        let right_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(top_layout[1]);
        
        // Undo/Redo buttons
        let can_undo = self.undo_redo.can_undo();
        let can_redo = self.undo_redo.can_redo();
        let undo_text = if can_undo { "Undo (Ctrl+U)" } else { "Undo (--)" };
        let redo_text = if can_redo { "Redo (Ctrl+R)" } else { "Redo (--)" };
        
        let controls = Paragraph::new(format!("{}\n{}", undo_text, redo_text))
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title("Controls"));
        frame.render_widget(controls, right_layout[0]);
        
        // Transaction controls
        let trans_text = if self.in_transaction {
            "In Transaction\nCtrl+T: End\nCtrl+C: Cancel"
        } else {
            "Ctrl+T: Start\ntransaction"
        };
        let transactions = Paragraph::new(trans_text)
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title("Transactions"));
        frame.render_widget(transactions, right_layout[1]);
        
        // Undo/Redo Stack
        let mut stack_items = Vec::new();
        
        if let Some(history) = self.undo_redo.get_history() {
            for entry in history {
                let style = match entry.tense {
                    sundo::Tenses::Past => Style::default().fg(Color::Gray),
                    sundo::Tenses::Present => Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD),
                    sundo::Tenses::Future => Style::default().fg(Color::DarkGray),
                };
                
                let label = match entry.tense {
                    sundo::Tenses::Past => format!("{} (past)", entry.description),
                    sundo::Tenses::Present => format!("{} (present)", entry.description),
                    sundo::Tenses::Future => format!("{} (future)", entry.description),
                };
                
                stack_items.push(ListItem::new(label).style(style));
            }
        }
        
        let stack = List::new(stack_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Undo/Redo Stack"));
        frame.render_widget(stack, right_layout[2]);
    }
}

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let result = App::default().run(terminal);
    ratatui::restore();
    result
}
