//! Repeatable comparison against the former persisted-key implementation.
//! Run: cargo run -p name-policy --example key_profile --release --locked --offline
use name_policy::profiling::materialized_key;
use std::{hint::black_box, time::Instant};

fn elapsed(operation: fn(&str) -> String, inputs: &[String], rounds: usize) -> u128 {
    let start = Instant::now();
    for _ in 0..rounds {
        for input in inputs {
            black_box(operation(black_box(input)));
        }
    }
    start.elapsed().as_nanos()
}

fn main() {
    let rounds = 30_000;
    let corpus = [
        (
            "ascii",
            vec!["Report-2026.TXT".into(), "README.md".repeat(20)],
        ),
        (
            "multilingual",
            vec!["报告（终稿）.docx".into(), "Straße.Σςσ.İı.각".repeat(6)],
        ),
        (
            "combining",
            vec![
                "A\u{315}\u{300}\u{5ae}\u{301}".repeat(8),
                "A\u{301}".repeat(70),
            ],
        ),
    ];
    println!("case,lookups,materialized_ns,streamed_ns,streamed_over_materialized");
    for (label, inputs) in corpus {
        for input in &inputs {
            assert_eq!(materialized_key(input), name_policy::lookup_key(input));
        }
        // Alternate ordering and report medians to limit warmup/order noise.
        let mut old = [0; 5];
        let mut new = [0; 5];
        for index in 0..5 {
            if index % 2 == 0 {
                old[index] = elapsed(materialized_key, &inputs, rounds);
                new[index] = elapsed(name_policy::lookup_key, &inputs, rounds);
            } else {
                new[index] = elapsed(name_policy::lookup_key, &inputs, rounds);
                old[index] = elapsed(materialized_key, &inputs, rounds);
            }
        }
        old.sort_unstable();
        new.sort_unstable();
        println!(
            "{label},{},{},{},{:.3}",
            rounds * inputs.len(),
            old[2],
            new[2],
            new[2] as f64 / old[2] as f64,
        );
    }
}
