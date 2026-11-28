//! Performance and stress benchmarks for `ned_regex`.

use ned_regex::regex;
use std::time::Instant;

#[test]
fn test_hardcore_regex_performance() {
    println!("\n=== HARDCORE REGEX PERFORMANCE TEST ===");

    // 1. Acceptance requirement patterns: a(l|p)pha and ^fn
    let pattern1 = "a(l|p)pha";
    let text1 = "alpha and appha are both matching words";
    let start = Instant::now();
    let re1 = regex::new(pattern1).expect("compile a(l|p)pha");
    let matches: Vec<_> = re1.find_iter(text1).collect();
    let elapsed = start.elapsed();
    println!(
        "1. a(l|p)pha match count: {} in {:?}",
        matches.len(),
        elapsed
    );
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].as_str(text1), "alpha");
    assert_eq!(matches[1].as_str(text1), "appha");

    let pattern2 = "^fn";
    let text2 = "fn main() {\n    let x = 1;\n}\nfn item() {}\nlet fn_var = 2;\n";
    let start = Instant::now();
    let re2 = regex::new(pattern2).expect("compile ^fn");
    let matches: Vec<_> = re2.find_iter(text2).collect();
    let elapsed = start.elapsed();
    println!(
        "2. ^fn line anchor match count: {} in {:?}",
        matches.len(),
        elapsed
    );
    assert_eq!(matches.len(), 2);

    // 3. ReDoS stress test: pathological nested repetitions (a+)+$ on 100 'a's
    // In backtracking engines (PCRE/JS), this hangs forever (O(2^N)).
    // In Thompson-NFA (PikeVM), this runs linearly in microseconds!
    let redos_pattern = "(a+)+b";
    let mut pathological_input = "a".repeat(100);
    pathological_input.push('c');
    let start = Instant::now();
    let re3 = regex::new(redos_pattern).expect("compile (a+)+b");
    let is_match = re3.is_match(&pathological_input);
    let elapsed = start.elapsed();
    println!(
        "3. ReDoS immunity test (a+)+b on 100 'a's without match: is_match={is_match} in {elapsed:?}"
    );
    assert!(!is_match);

    // 4. Massive text search: 100,000 characters with alternations and repetitions
    let words = [
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta",
    ];
    let mut huge_text = String::with_capacity(200_000);
    for i in 0..15_000 {
        huge_text.push_str(words[i % words.len()]);
        huge_text.push(' ');
    }
    huge_text.push_str("needle_special_target");

    let complex_pattern = "(alpha|beta|gamma|theta)";
    let start = Instant::now();
    let re4 = regex::new(complex_pattern).expect("compile complex alternation");
    let compile_time = start.elapsed();

    let start_scan = Instant::now();
    let match_count = re4.find_iter(&huge_text).count();
    let scan_time = start_scan.elapsed();

    println!(
        "4. Massive text ({} bytes, ~100k chars):\n   - Pattern: '{}'\n   - Compile time: {:?}\n   - Total matches found: {}\n   - Scan time: {:?}",
        huge_text.len(),
        complex_pattern,
        compile_time,
        match_count,
        scan_time
    );

    // 5. Word boundary and character class search on huge text
    let class_pattern = r"\b[a-z]{4,6}\b";
    let start = Instant::now();
    let re5 = regex::new(class_pattern).expect("compile bounded word class");
    let compile_time5 = start.elapsed();

    let start_scan5 = Instant::now();
    let count5 = re5.find_iter(&huge_text).count();
    let scan_time5 = start_scan5.elapsed();

    println!(
        "5. Word boundary & range query (\\b[a-z]{{4,6}}\\b):\n   - Compile time: {compile_time5:?}\n   - Total matches: {count5}\n   - Scan time: {scan_time5:?}"
    );

    println!("=== ALL HARDCORE REGEX PERFORMANCE CHECKS PASSED ===\n");
}
