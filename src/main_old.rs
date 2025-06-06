use dashmap::DashMap;
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

const POSSIBLE_LETTERS: [Letter; 14] = [
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

const SCHEMA: [[i32; 5]; 5] = [
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [1, 2, 1, 3, 1],
    [1, 1, 1, 2, 1],
];

type FiveWord = [char; 5];
type ValidWord = (FiveWord, bool); // (word, wildcard_used)
fn print_five_word(word: FiveWord) -> String {
    let mut s = String::new();
    for c in word.iter() {
        s.push(*c);
    }
    s
}

fn validate_sub_board(v: &[&Box<ValidWord>]) -> bool {
    // Check if the sub-board is valid
    // This function ensures that the sub-board can be formed with the available letters

    let mut used = HashMap::new();
    let mut any_used = 0;
    for word in v {
        let wc = word.1; // Wildcard used
        let word = word.0;
        for &c in word.iter() {
            *used.entry(c).or_insert(0) += 1;
        }
        if wc {
            any_used += 1;
        }
    }
    for l in &POSSIBLE_LETTERS {
        if used.get(&l.ch).unwrap_or(&0) > &l.num {
            return false;
        }
    }
    if any_used > 1 {
        return false;
    }
    true
}

fn generate_k_sets(valid_words: Vec<&Box<ValidWord>>, k: i32) -> Vec<Vec<&Box<ValidWord>>> {
    // Generate all combinations of k valid words from the valid_words vector, parallelized with Rayon
    let n = valid_words.len();
    if k == 0 {
        return vec![Vec::new()];
    }
    (0..n)
        .into_par_iter()
        .map(|i| {
            let mut sub_combinations = generate_k_sets(valid_words[i + 1..].to_vec(), k - 1);
            let mut results = Vec::new();
            for sub in &mut sub_combinations {
                if !validate_sub_board(sub) {
                    continue;
                }
                sub.insert(0, valid_words[i]);
                results.push(sub.clone());
            }
            results
        })
        .flatten()
        .collect()
}

fn generate_k_sets_memo<'a>(
    valid_words: Arc<Vec<&'a Box<ValidWord>>>,
    k: i32,
    start: usize,
) -> Vec<Vec<&'a Box<ValidWord>>> {
    if k == 0 {
        return vec![Vec::new()];
    }
    let n = valid_words.len();
    let batch_size = 32; // Tune this for your workload
    let batch_starts: Vec<usize> = (start..n).step_by(batch_size).collect();
    let results: Vec<Vec<&Box<ValidWord>>> = batch_starts
        .into_par_iter()
        .flat_map(|batch_start| {
            let mut local_results = Vec::new();
            let batch_end = (batch_start + batch_size).min(n);
            for i in batch_start..batch_end {
                let sub_combinations = generate_k_sets_memo(valid_words.clone(), k - 1, i + 1);
                for mut sub in sub_combinations {
                    if !validate_sub_board(&sub) {
                        continue;
                    }
                    sub.push(valid_words[i]); // push at end, no insert at front
                    // if validate_sub_board(&sub) {
                    local_results.push(sub);
                    // }
                }
            }
            local_results
        })
        .collect();
    results
}

fn main() {
    // Hardcoded letter bag
    let mut letter_bag = Vec::new();
    let mut letter_scores = HashMap::new();
    for l in &POSSIBLE_LETTERS {
        for _ in 0..l.num {
            letter_bag.push(l.ch);
        }
        letter_scores.insert(l.ch, l.score);
    }

    // Read words from file
    let file = File::open("sgb-words-mini.txt").expect("data.txt not found");
    let reader = BufReader::new(file);
    let words: HashSet<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .map(|w| w.trim().to_uppercase())
        .filter(|w| w.len() == 5)
        .collect();
    let words_arc = Arc::new(words);

    // Generate all possible valid rows
    let valid_words: Vec<Box<ValidWord>> = words_arc
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
            // If we reach here, the word is valid
            Some(Box::new((
                [
                    word.chars().nth(0).unwrap(),
                    word.chars().nth(1).unwrap(),
                    word.chars().nth(2).unwrap(),
                    word.chars().nth(3).unwrap(),
                    word.chars().nth(4).unwrap(),
                ],
                wildcard_used,
            )))
        })
        .collect();
    let valid_words_copy: Vec<&Box<ValidWord>> = valid_words.iter().collect();
    print!("Copied {} valid words to memory. ", valid_words.len());
    // flush io
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    // Now, since we have all valid words, we can make a collection of all valid combinations of words into 5 rows
    const K: i32 = 3;
    // let valid_sets = generate_k_sets(valid_words_copy, K);
    let valid_sets = generate_k_sets_memo(valid_words_copy.into(), K, 0);
    println!("Total valid sets of {K} rows found: {}", valid_sets.len());

    // exit early
    return;

    println!("Total valid rows found: {}", valid_words.len());
    for (i, word) in valid_words.iter().take(5).enumerate() {
        println!("Row {}: {}", i, print_five_word(word.0));
    }
    if valid_words.len() > 5 {
        println!("... and more rows available.");
    }

    // Brute force all possible 5x5 boards (parallel, batched)
    let valid_rows_arc = Arc::new(valid_words);
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
                    let board = [
                        &valid_rows_arc[i0],
                        &valid_rows_arc[i1],
                        &valid_rows_arc[i2],
                        &valid_rows_arc[i3],
                        &valid_rows_arc[i4],
                    ];
                    // Check bag usage
                    if !validate_sub_board(&board) {
                        continue;
                    }
                    // // Check columns
                    // let mut cols = vec![String::new(); 5];
                    // for row in &board {
                    //     for (i, c) in row.0.chars().enumerate() {
                    //         cols[i].push(c);
                    //     }
                    // }
                    // if !cols.iter().all(|col| words_arc.contains(col)) {
                    //     continue;
                    // }
                    // Score
                    let mut score = 0;
                    for (r, word) in board.iter().enumerate() {
                        let word = word.0;
                        for (c, ch) in word.iter().enumerate() {
                            score += *letter_scores.get(&ch).unwrap_or(&0) * SCHEMA[r][c];
                        }
                    }
                    if score > local_best.0 {
                        local_best = (score, board.iter().map(|word| word.0).collect::<Vec<_>>());
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
            println!("{}", print_five_word(*row));
        }
        println!("Score: {}", best.0);
    } else {
        println!("No valid board found.");
    }
}
