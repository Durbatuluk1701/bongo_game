use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
struct Letter {
    ch: char,
    num: usize,
    score: i32,
}

const SCHEMA: [[i32; 5]; 5] = [
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [1, 2, 1, 3, 1],
    [1, 1, 1, 2, 1],
];

fn main() {
    // Hardcoded letter bag
    let possible_letters = vec![
        Letter {
            ch: 'G',
            num: 1,
            score: 45,
        },
        Letter {
            ch: 'B',
            num: 3,
            score: 50,
        },
        Letter {
            ch: 'M',
            num: 1,
            score: 35,
        },
        Letter {
            ch: 'D',
            num: 1,
            score: 30,
        },
        Letter {
            ch: 'N',
            num: 2,
            score: 20,
        },
        Letter {
            ch: 'U',
            num: 1,
            score: 15,
        },
        Letter {
            ch: 'L',
            num: 1,
            score: 9,
        },
        Letter {
            ch: 'T',
            num: 2,
            score: 10,
        },
        Letter {
            ch: 'O',
            num: 2,
            score: 7,
        },
        Letter {
            ch: 'R',
            num: 2,
            score: 7,
        },
        Letter {
            ch: 'S',
            num: 3,
            score: 5,
        },
        Letter {
            ch: 'A',
            num: 4,
            score: 5,
        },
        Letter {
            ch: 'E',
            num: 2,
            score: 5,
        },
        Letter {
            ch: '*',
            num: 1,   // Wildcard
            score: 0, // Wildcard has no score
        },
    ];
    let mut letter_bag = Vec::new();
    let mut letter_scores = HashMap::new();
    for l in &possible_letters {
        for _ in 0..l.num {
            letter_bag.push(l.ch);
        }
        letter_scores.insert(l.ch, l.score);
    }

    // Read words from file
    let file = File::open("sgb-words.txt").expect("data.txt not found");
    let reader = BufReader::new(file);
    let words: HashSet<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .map(|w| w.trim().to_uppercase())
        .filter(|w| w.len() == 5)
        .collect();
    let words_arc = Arc::new(words);

    // Generate all possible valid rows
    let valid_rows: Vec<(String, bool)> = words_arc
        .par_iter()
        .filter_map(|word| {
            let mut bag = letter_bag.clone();
            let mut wildcard_used = false;
            for c in word.chars() {
                if let Some(pos) = bag.iter().position(|&x| x == c) {
                    bag.remove(pos);
                } else if !wildcard_used {
                    if let Some(pos) = bag.iter().position(|&x| x == '*') {
                        bag.remove(pos);
                        wildcard_used = true;
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            Some((word.clone(), wildcard_used))
        })
        .collect();
    println!("Total valid rows found: {}", valid_rows.len());
    for (i, (w, _)) in valid_rows.iter().take(5).enumerate() {
        println!("Row {}: {}", i, w);
    }
    if valid_rows.len() > 5 {
        println!("... and more rows available.");
    }

    // Brute force all possible 5x5 boards (parallel, batched)
    let valid_rows_arc = Arc::new(valid_rows);
    let total = valid_rows_arc.len().pow(5);
    println!("Total combinations to check: {}", total);
    let batch_size = 1_000_000usize;
    let num_batches = (total + batch_size - 1) / batch_size;
    let progress = Arc::new(Mutex::new(0usize));
    let best = (0..num_batches)
        .into_par_iter()
        .map_init(
            || progress.clone(),
            |progress, batch_idx| {
                let mut local_best = (0, Vec::new());
                let batch_start = batch_idx * batch_size;
                let batch_end = ((batch_idx + 1) * batch_size).min(total);
                for idx in batch_start..batch_end {
                    let n = valid_rows_arc.len();
                    let i0 = idx / (n * n * n * n) % n;
                    let i1 = idx / (n * n * n) % n;
                    let i2 = idx / (n * n) % n;
                    let i3 = idx / n % n;
                    let i4 = idx % n;
                    let board = vec![
                        &valid_rows_arc[i0],
                        &valid_rows_arc[i1],
                        &valid_rows_arc[i2],
                        &valid_rows_arc[i3],
                        &valid_rows_arc[i4],
                    ];
                    // Check bag usage
                    let mut used = HashMap::new();
                    let mut any_used = 0;
                    for (word, wc) in &board {
                        for c in word.chars() {
                            *used.entry(c).or_insert(0) += 1;
                        }
                        if *wc {
                            any_used += 1;
                        }
                    }
                    for l in &possible_letters {
                        if used.get(&l.ch).unwrap_or(&0) > &l.num {
                            continue;
                        }
                    }
                    if any_used > 1 {
                        continue;
                    }
                    // Check columns
                    let mut cols = vec![String::new(); 5];
                    for row in &board {
                        for (i, c) in row.0.chars().enumerate() {
                            cols[i].push(c);
                        }
                    }
                    if !cols.iter().all(|col| words_arc.contains(col)) {
                        continue;
                    }
                    // Score
                    let mut score = 0;
                    for (r, (word, _)) in board.iter().enumerate() {
                        for (c, ch) in word.chars().enumerate() {
                            score += *letter_scores.get(&ch).unwrap_or(&0) * SCHEMA[r][c];
                        }
                    }
                    if score > local_best.0 {
                        local_best = (
                            score,
                            board.iter().map(|(w, _)| w.clone()).collect::<Vec<_>>(),
                        );
                    }
                }
                // Progress bar update (mutex)
                {
                    let mut done = progress.lock().unwrap();
                    *done += 1;
                    let percent = (*done as f64) * 100.0 / (num_batches as f64);
                    let bar_len = 40;
                    let filled = (percent / 100.0 * bar_len as f64).round() as usize;
                    let bar: String = "#".repeat(filled) + &"-".repeat(bar_len - filled);
                    print!("\r[{}] {:.2}% ({} / {})", bar, percent, *done, num_batches);
                    use std::io::Write;
                    std::io::stdout().flush().unwrap();
                }
                local_best
            },
        )
        .max_by_key(|(score, _)| *score)
        .unwrap();
    println!("");
    if best.0 > 0 {
        println!("Best board:");
        for row in &best.1 {
            println!("{}", row);
        }
        println!("Score: {}", best.0);
    } else {
        println!("No valid board found.");
    }
}
