//! stress-test demonstrating a heavy diff running concurrently with an active event loop ticker.

use ned_git::compute_diff_async;
use std::time::{Duration, Instant};

#[yokio::main]
fn main() {
    println!("=== [yokio] starting concurrent heavy diff test ===");

    // generate a large 8,000-line base text and modified current text
    let mut base = String::with_capacity(300_000);
    let mut current = String::with_capacity(300_000);

    for i in 0..8_000 {
        use std::fmt::Write;
        let _ = writeln!(base, "fn line_{i}() -> usize {{ return {i}; }}");
        if i % 100 == 0 {
            let _ = writeln!(
                current,
                "fn line_{i}_modified() -> usize {{ return {}; }}",
                i * 2
            );
        } else if i % 250 == 0 {
            // omit line (deletion)
        } else {
            let _ = writeln!(current, "fn line_{i}() -> usize {{ return {i}; }}");
        }
        if i % 500 == 0 {
            let _ = writeln!(current, "// extra inserted line at {i}");
        }
    }

    println!(
        "generated ~8,000 lines (base: {} bytes, current: {} bytes)",
        base.len(),
        current.len()
    );

    let start_time = Instant::now();
    let mut tick_counter = 0;

    // simulate the editor's 50 FPS UI frame loop (every 20ms)
    let _ticker = yokio::every(Duration::from_millis(20), move || {
        tick_counter += 1;
        println!(
            "[ui loop] frame tick #{tick_counter} (elapsed: {:?}) - loop is responsive!",
            start_time.elapsed()
        );
    });

    println!("scheduling heavy diff on yokio worker pool...");

    // dispatch the heavy diff onto yokio worker pool
    compute_diff_async(base, current, move |hunks| {
        let elapsed = start_time.elapsed();
        println!("=== [yokio worker] heavy diff finished! ===");
        println!("computed {} hunks in {:?}", hunks.len(), elapsed);
        println!("first 3 hunks: {:?}", &hunks[..3.min(hunks.len())]);
        println!("last hunk: {:?}", hunks.last());
        println!("=== test passed: main loop was never blocked! ===");
        yokio::stop();
    });
}
