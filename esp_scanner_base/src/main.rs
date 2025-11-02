//! # File Scanner
//!
//! CLI for scanning ICS files with file system collectors

use ics_sdk::prelude::*;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ICS File Scanner ===\n");

    let args: Vec<String> = env::args().collect();
    let target_path = if args.len() > 1 {
        &args[1]
    } else {
        println!("Usage: {} <file.ics or directory/>", args[0]);
        return Ok(());
    };

    // Create registry with file system strategies
    let registry = create_scanner_registry()?;
    let mut scanner = IcsScanner::new(registry)?;

    // Scan based on path type
    if std::path::Path::new(target_path).is_dir() {
        let batch_result = scanner.scan_directory(target_path)?;
        println!("{}", batch_result.summary());

        let json = batch_result.to_json()?;
        std::fs::write("batch_scan_results.json", &json)?;
        println!("\n[OK] Results saved to: batch_scan_results.json");
    } else {
        let result = scanner.scan_file(target_path)?;
        println!(
            "Scan Status: {}",
            if result.results.passed {
                "PASS"
            } else {
                "FAIL"
            }
        );
        println!("Total Criteria: {}", result.results.check.total_criteria);
        println!("Findings: {}", result.results.findings.len());

        let json = result.to_json()?;
        std::fs::write("scan_result.json", &json)?;
        println!("\n[OK] Results saved to: scan_result.json");
    }

    Ok(())
}
