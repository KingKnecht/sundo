/// Memory Performance Tests Demo
/// 
/// This demo runs various memory performance tests for the sundo library.
/// These tests measure memory usage patterns for different undo/redo scenarios.
/// 
/// IMPORTANT: Run with --release flag for accurate measurements!

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use sundo::{Actions, UndoRedo};
use im::Vector as ImVector;

// Custom allocator to track memory usage
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = unsafe { System.alloc(layout) };
        if !ret.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn get_allocated_bytes() -> usize {
    ALLOCATED.load(Ordering::SeqCst)
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

// Large model struct (~100MB when filled with data)
#[derive(Clone, Debug)]
struct LargeModel {
    data: Vec<i32>,
    version: u32,
}

impl LargeModel {
    fn new(size_mb: usize) -> Self {
        let elements = (size_mb * 1024 * 1024) / 4;
        let data: Vec<i32> = (0..elements as i32).collect();
        Self { data, version: 0 }
    }

    fn modify(&self, increment: i32) -> Self {
        let mut new_data = self.data.clone();
        for i in 0..1000.min(new_data.len()) {
            new_data[i] += increment;
        }
        Self {
            data: new_data,
            version: self.version + 1,
        }
    }

    fn size_estimate(&self) -> usize {
        self.data.len() * std::mem::size_of::<i32>()
    }
}

// Immutable model using im::Vector with structural sharing
#[derive(Clone, Debug)]
struct ImmutableModel {
    data: ImVector<i32>,
    version: u32,
}

impl ImmutableModel {
    fn new(size_mb: usize) -> Self {
        let elements = (size_mb * 1024 * 1024) / 4;
        let data: ImVector<i32> = (0..elements as i32).collect();
        Self { data, version: 0 }
    }

    fn modify(&self, increment: i32) -> Self {
        let mut new_data = self.data.clone();
        for i in 0..1000.min(new_data.len()) {
            new_data[i] += increment;
        }
        Self {
            data: new_data,
            version: self.version + 1,
        }
    }

    fn size_estimate(&self) -> usize {
        self.data.len() * std::mem::size_of::<i32>()
    }
}

fn run_memory_test() {
    println!("=== Memory Performance Test ===\n");
    
    let baseline = get_allocated_bytes();
    println!("Baseline memory: {}", format_bytes(baseline));
    
    println!("\n--- Creating 100MB model ---");
    let model = LargeModel::new(100);
    let after_model = get_allocated_bytes();
    let model_size = after_model - baseline;
    println!("After model creation: {} (model: {})", 
             format_bytes(after_model), 
             format_bytes(model_size));
    println!("Model estimate: {}", format_bytes(model.size_estimate()));
    
    println!("\n--- Push initial state to UndoRedo ---");
    let mut undo_redo = UndoRedo::<LargeModel>::new();
    undo_redo.push(model, "Initial state".to_string());
    let after_push = get_allocated_bytes();
    println!("After push: {} (+{})", 
             format_bytes(after_push),
             format_bytes(after_push - after_model));
    
    println!("\n--- Push 5 modifications ---");
    for i in 1..=5 {
        undo_redo.update(
            |m| m.modify(i),
            format!("Modification {}", i)
        );
    }
    let after_5_mods = get_allocated_bytes();
    let mods_overhead = after_5_mods - after_push;
    println!("After 5 modifications: {} (+{})", 
             format_bytes(after_5_mods),
             format_bytes(mods_overhead));
    println!("History length: {}", undo_redo.get_history_len());
    println!("Average overhead per entry: {}", 
             format_bytes(mods_overhead / 5));
    
    println!("\n--- Undo 3 times ---");
    undo_redo.undo();
    undo_redo.undo();
    undo_redo.undo();
    let after_undo = get_allocated_bytes();
    println!("After 3 undos: {} (change: {})", 
             format_bytes(after_undo),
             if after_undo > after_5_mods {
                 format!("+{}", format_bytes(after_undo - after_5_mods))
             } else {
                 format_bytes(after_5_mods - after_undo)
             });
    
    println!("\n--- Redo 2 times ---");
    undo_redo.redo();
    undo_redo.redo();
    let after_redo = get_allocated_bytes();
    println!("After 2 redos: {} (change: {})", 
             format_bytes(after_redo),
             if after_redo > after_undo {
                 format!("+{}", format_bytes(after_redo - after_undo))
             } else {
                 format_bytes(after_undo - after_redo)
             });
    
    println!("\n--- Push new modification (truncates future) ---");
    let before_truncate = get_allocated_bytes();
    undo_redo.update(
        |m| m.modify(99),
        "New branch".to_string()
    );
    let after_truncate = get_allocated_bytes();
    println!("Before truncate: {}", format_bytes(before_truncate));
    println!("After truncate: {} (freed: {})", 
             format_bytes(after_truncate),
             format_bytes(before_truncate.saturating_sub(after_truncate)));
    println!("History length: {}", undo_redo.get_history_len());
    
    println!("\n--- Test transaction (10 modifications) ---");
    let before_transaction = get_allocated_bytes();
    undo_redo.start_transaction();
    for i in 1..=10 {
        undo_redo.update(
            |m| m.modify(i),
            format!("Transaction step {}", i)
        );
    }
    let during_transaction = get_allocated_bytes();
    println!("During transaction: {} (+{})", 
             format_bytes(during_transaction),
             format_bytes(during_transaction - before_transaction));
    println!("History length during: {}", undo_redo.get_history_len());
    
    undo_redo.end_transaction("10 modifications".to_string());
    let after_transaction = get_allocated_bytes();
    println!("After end_transaction: {} (freed: {})", 
             format_bytes(after_transaction),
             format_bytes(during_transaction.saturating_sub(after_transaction)));
    println!("History length after: {}", undo_redo.get_history_len());
    
    println!("\n--- Test cancel transaction (5 modifications) ---");
    undo_redo.start_transaction();
    for i in 1..=5 {
        undo_redo.update(
            |m| m.modify(i * 10),
            format!("To be cancelled {}", i)
        );
    }
    let before_cancel = get_allocated_bytes();
    undo_redo.cancel_transaction();
    let after_cancel = get_allocated_bytes();
    println!("Before cancel: {}", format_bytes(before_cancel));
    println!("After cancel: {} (freed: {})", 
             format_bytes(after_cancel),
             format_bytes(before_cancel.saturating_sub(after_cancel)));
    
    println!("\n--- Drop UndoRedo ---");
    let before_drop = get_allocated_bytes();
    drop(undo_redo);
    let after_drop = get_allocated_bytes();
    println!("Before drop: {}", format_bytes(before_drop));
    println!("After drop: {} (freed: {})", 
             format_bytes(after_drop),
             format_bytes(before_drop.saturating_sub(after_drop)));
    
    println!("\n=== Summary ===");
    println!("Baseline: {}", format_bytes(baseline));
    println!("Peak usage: {}", format_bytes(during_transaction));
    println!("Final: {}", format_bytes(after_drop));
    println!("Total overhead: {}", format_bytes(during_transaction - baseline));
}

fn run_rc_sharing_test() {
    println!("\n\n=== Rc Sharing Test ===\n");
    println!("This test demonstrates that Rc avoids unnecessary copying\n");
    
    let baseline = get_allocated_bytes();
    
    let model = LargeModel::new(50);
    let after_model = get_allocated_bytes();
    println!("Created 50MB model: {} (+{})", 
             format_bytes(after_model),
             format_bytes(after_model - baseline));
    
    let mut undo_redo = UndoRedo::<LargeModel>::new();
    let rc1 = undo_redo.push(model, "Initial".to_string());
    let after_push = get_allocated_bytes();
    println!("Pushed to UndoRedo: {} (+{})", 
             format_bytes(after_push),
             format_bytes(after_push - after_model));
    
    let rc2 = undo_redo.present().unwrap();
    let after_rc2 = get_allocated_bytes();
    let rc3 = undo_redo.present().unwrap();
    let after_rc3 = get_allocated_bytes();
    
    println!("\nGot 2nd Rc reference: {} (+{})", 
             format_bytes(after_rc2),
             format_bytes(after_rc2 - after_push));
    println!("Got 3rd Rc reference: {} (+{})", 
             format_bytes(after_rc3),
             format_bytes(after_rc3 - after_rc2));
    
    println!("\nAll Rc pointers point to same data: {}", 
             std::rc::Rc::as_ptr(&rc1) == std::rc::Rc::as_ptr(&rc2) && 
             std::rc::Rc::as_ptr(&rc2) == std::rc::Rc::as_ptr(&rc3));
    
    drop(rc1);
    drop(rc2);
    drop(rc3);
    drop(undo_redo);
    
    let final_mem = get_allocated_bytes();
    println!("\nAfter dropping all: {}", format_bytes(final_mem));
}

fn run_comparison_test() {
    println!("\n\n=== Comparison: With vs Without UndoRedo ===\n");
    
    println!("--- Without UndoRedo (Vec of models) ---");
    let baseline = get_allocated_bytes();
    
    let mut history_vec: Vec<LargeModel> = Vec::new();
    let model = LargeModel::new(50);
    history_vec.push(model);
    
    for i in 1..=5 {
        let new_model = history_vec.last().unwrap().modify(i);
        history_vec.push(new_model);
    }
    
    let vec_memory = get_allocated_bytes();
    let vec_overhead = vec_memory - baseline;
    println!("Memory with Vec of 6 models: {} (overhead: {})", 
             format_bytes(vec_memory),
             format_bytes(vec_overhead));
    println!("Average per entry: {}", format_bytes(vec_overhead / 6));
    
    drop(history_vec);
    
    println!("\n--- With UndoRedo (using Rc) ---");
    let baseline = get_allocated_bytes();
    
    let mut undo_redo = UndoRedo::<LargeModel>::new();
    let model = LargeModel::new(50);
    undo_redo.push(model, "Initial".to_string());
    
    for i in 1..=5 {
        undo_redo.update(
            |m| m.modify(i),
            format!("Change {}", i)
        );
    }
    
    let undo_memory = get_allocated_bytes();
    let undo_overhead = undo_memory - baseline;
    println!("Memory with UndoRedo of 6 entries: {} (overhead: {})", 
             format_bytes(undo_memory),
             format_bytes(undo_overhead));
    println!("Average per entry: {}", format_bytes(undo_overhead / 6));
    
    drop(undo_redo);
    
    println!("\n--- Summary ---");
    println!("Vec approach overhead: {}", format_bytes(vec_overhead));
    println!("UndoRedo approach overhead: {}", format_bytes(undo_overhead));
    println!("Memory savings: {} ({:.1}%)", 
             format_bytes(vec_overhead.saturating_sub(undo_overhead)),
             if vec_overhead > 0 {
                 (vec_overhead.saturating_sub(undo_overhead)) as f64 / vec_overhead as f64 * 100.0
             } else {
                 0.0
             });
}

fn run_immutable_comparison_test() {
    println!("\n\n=== Immutable Vec vs Standard Vec Comparison ===\n");
    println!("This test compares memory usage of im::Vector (structural sharing)");
    println!("vs standard Vec (full copy on modification)\n");
    
    println!("--- Standard Vec (full copy on modify) ---");
    let baseline = get_allocated_bytes();
    
    let mut undo_redo_vec = UndoRedo::<LargeModel>::new();
    let model = LargeModel::new(50);
    undo_redo_vec.push(model, "Initial".to_string());
    
    let after_first = get_allocated_bytes();
    println!("After initial push: {} (+{})", 
             format_bytes(after_first),
             format_bytes(after_first - baseline));
    
    for i in 1..=10 {
        undo_redo_vec.update(
            |m| m.modify(i),
            format!("Change {}", i)
        );
    }
    
    let vec_memory = get_allocated_bytes();
    let vec_total = vec_memory - baseline;
    println!("After 10 modifications: {} (total: {})", 
             format_bytes(vec_memory),
             format_bytes(vec_total));
    println!("History length: {}", undo_redo_vec.get_history_len());
    println!("Average per entry: {}", format_bytes(vec_total / 11));
    
    drop(undo_redo_vec);
    
    println!("\n--- Immutable im::Vector (structural sharing) ---");
    let baseline = get_allocated_bytes();
    
    let mut undo_redo_im = UndoRedo::<ImmutableModel>::new();
    let im_model = ImmutableModel::new(50);
    undo_redo_im.push(im_model, "Initial".to_string());
    
    let after_first = get_allocated_bytes();
    println!("After initial push: {} (+{})", 
             format_bytes(after_first),
             format_bytes(after_first - baseline));
    
    for i in 1..=10 {
        undo_redo_im.update(
            |m| m.modify(i),
            format!("Change {}", i)
        );
    }
    
    let im_memory = get_allocated_bytes();
    let im_total = im_memory - baseline;
    println!("After 10 modifications: {} (total: {})", 
             format_bytes(im_memory),
             format_bytes(im_total));
    println!("History length: {}", undo_redo_im.get_history_len());
    println!("Average per entry: {}", format_bytes(im_total / 11));
    
    drop(undo_redo_im);
    
    println!("\n--- Comparison Summary ---");
    println!("Standard Vec total memory: {}", format_bytes(vec_total));
    println!("Immutable Vector total memory: {}", format_bytes(im_total));
    
    if vec_total > im_total {
        let savings = vec_total - im_total;
        let percent = (savings as f64 / vec_total as f64) * 100.0;
        println!("Memory saved with structural sharing: {} ({:.1}%)", 
                 format_bytes(savings), percent);
    } else {
        let overhead = im_total - vec_total;
        let percent = (overhead as f64 / vec_total as f64) * 100.0;
        println!("Memory overhead with im::Vector: {} ({:.1}%)", 
                 format_bytes(overhead), percent);
    }
    
    println!("\n🔍 Key Insight:");
    println!("Immutable data structures use 'structural sharing' - when you");
    println!("modify part of the data, only the changed parts are copied.");
    println!("Unchanged parts are shared between versions, dramatically");
    println!("reducing memory usage in undo/redo systems!");
}

fn run_detailed_immutable_test() {
    println!("\n\n=== Detailed Immutable Vector Analysis ===\n");
    
    println!("Creating 50MB model with im::Vector...");
    let baseline = get_allocated_bytes();
    
    let mut undo_redo = UndoRedo::<ImmutableModel>::new();
    let model = ImmutableModel::new(50);
    let after_model = get_allocated_bytes();
    
    println!("Model created: {} (+{})", 
             format_bytes(after_model),
             format_bytes(after_model - baseline));
    
    undo_redo.push(model, "Initial state".to_string());
    let after_push = get_allocated_bytes();
    println!("After push to UndoRedo: {} (+{})", 
             format_bytes(after_push),
             format_bytes(after_push - after_model));
    
    println!("\nApplying modifications (each modifies 1000 elements):");
    for i in 1..=20 {
        let before = get_allocated_bytes();
        undo_redo.update(
            |m| m.modify(i),
            format!("Modification {}", i)
        );
        let after = get_allocated_bytes();
        let delta = after.saturating_sub(before);
        
        if i <= 5 || i > 15 {
            println!("  Modification {}: +{}", i, format_bytes(delta));
        } else if i == 6 {
            println!("  ...");
        }
    }
    
    let after_all = get_allocated_bytes();
    let total_overhead = after_all - baseline;
    
    println!("\n--- Final Statistics ---");
    println!("Total memory used: {}", format_bytes(after_all));
    println!("Total overhead: {}", format_bytes(total_overhead));
    println!("History entries: {}", undo_redo.get_history_len());
    println!("Average overhead per modification: {}", 
             format_bytes(total_overhead / undo_redo.get_history_len()));
    println!("Base model size (estimated): 50.00 MB");
    println!("Overhead ratio: {:.2}x of base model", 
             total_overhead as f64 / (50.0 * 1024.0 * 1024.0));
    
    println!("\n--- Testing Undo/Redo (Rc already shares data) ---");
    let before_undo = get_allocated_bytes();
    
    for _ in 0..10 {
        undo_redo.undo();
    }
    let after_undo = get_allocated_bytes();
    println!("After 10 undos: {} (Δ: {})", 
             format_bytes(after_undo),
             format_bytes(after_undo.saturating_sub(before_undo)));
    
    for _ in 0..5 {
        undo_redo.redo();
    }
    let after_redo = get_allocated_bytes();
    println!("After 5 redos: {} (Δ: {})", 
             format_bytes(after_redo),
             format_bytes(after_redo.saturating_sub(after_undo)));
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║         Sundo Memory Performance Tests                        ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Test 1: Basic Memory Usage");
    println!("═══════════════════════════════════════════════════════════════");
    run_memory_test();
    println!();
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Test 2: Rc Sharing Efficiency");
    println!("═══════════════════════════════════════════════════════════════");
    run_rc_sharing_test();
    println!();
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Test 3: Vec vs im::Vector Comparison");
    println!("═══════════════════════════════════════════════════════════════");
    run_comparison_test();
    println!();
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Test 4: Immutable Structure Comparison");
    println!("═══════════════════════════════════════════════════════════════");
    run_immutable_comparison_test();
    println!();
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Test 5: Detailed Immutable Analysis");
    println!("═══════════════════════════════════════════════════════════════");
    run_detailed_immutable_test();
    println!();
    
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║ All tests completed!                                          ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
}
