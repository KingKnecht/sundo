use undo_redo::mem_performance_test::{
    run_memory_test, 
    run_rc_sharing_test, 
    run_comparison_test,
    run_immutable_comparison_test,
    run_detailed_immutable_test
};

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  SUNDO Memory Performance Test Suite                      ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    
    run_memory_test();
    run_rc_sharing_test();
    run_comparison_test();
    run_immutable_comparison_test();
    run_detailed_immutable_test();
    
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  Tests Complete                                            ║");
    println!("╚════════════════════════════════════════════════════════════╝");
}
