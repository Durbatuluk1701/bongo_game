use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
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

const BONUS_WORD_INDS: [(usize, usize); 4] = [(0, 2), (1, 2), (2, 2), (3, 3)];

type FourWord = [char; 4];
type FiveWord = [char; 5];
type ValidWord = (FiveWord, Option<char>); // (word, wildcard_used)
fn print_five_word(word: FiveWord) -> String {
    word.iter().collect()
}

fn generate_k_sets(
    valid_words: Vec<&ValidWord>,
    k: i32,
    word_bag: Vec<char>,
) -> Vec<Vec<&ValidWord>> {
    // Generate all combinations of k valid words from the valid_words vector, parallelized with Rayon
    let n = valid_words.len();
    if k == 0 {
        return vec![Vec::new()];
    }
    (0..n)
        .into_par_iter()
        .map(|i| {
            let cur_valid_word = &valid_words[i];
            // Prune word bag
            let mut new_word_bag = word_bag.clone();
            for &c in cur_valid_word.0.iter() {
                if let Some(pos) = new_word_bag.iter().position(|&x| x == c) {
                    new_word_bag.remove(pos);
                } else if let Some(_ind) = cur_valid_word.1 {
                    // If wildcard is used, remove it
                    if let Some(pos) = new_word_bag.iter().position(|&x| x == '*') {
                        new_word_bag.remove(pos);
                    } else {
                        return vec![];
                    }
                } else {
                    return vec![];
                }
            }
            // Drop off other valid_words that are not valid for the current word_bag
            let next_valid_words = valid_words[i + 1..]
                .iter()
                .filter(|&&w| {
                    w.0.iter().all(|&c| {
                        new_word_bag
                            .iter()
                            .any(|&x| x == c || (w.1 == Some(c) && x == '*'))
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            generate_k_sets(next_valid_words, k - 1, new_word_bag)
                .into_iter()
                .map(|mut set| {
                    set.push(cur_valid_word);
                    set
                })
                .collect::<Vec<Vec<_>>>()
        })
        .flatten()
        .collect()
}

fn permute_board<'a>(board: &'a [&'a ValidWord]) -> Vec<Vec<&'a ValidWord>> {
    if board.len() != 5 {
        panic!("Board must have exactly 5 words.");
    }
    let mut result = Vec::new();
    let mut indices: Vec<usize> = (0..5).collect();
    loop {
        let mut current_set = Vec::with_capacity(5);
        for &i in &indices {
            current_set.push(board[i]);
        }
        result.push(current_set);

        // Generate next permutation
        let mut i = 4;
        while i > 0 && indices[i - 1] >= indices[i] {
            i -= 1;
        }
        if i == 0 {
            break; // No more permutations
        }
        let mut j = 4;
        while indices[j] <= indices[i - 1] {
            j -= 1;
        }
        indices.swap(i - 1, j);
        indices[i..].reverse();
    }
    result
}

fn score_board(
    board: &[&ValidWord],
    letter_scores: &HashMap<char, i32>,
    four_letter_words: &HashSet<FourWord>,
) -> i32 {
    let mut wildcard_letter = '*';
    for word in board.iter() {
        if let Some(wildchar) = word.1 {
            // If wildcard is used, we can use any letter in its place
            wildcard_letter = wildchar;
        }
    }
    // Now, find all places that letter is used in this board
    let all_wildcard_indices: Vec<(usize, usize)> = board
        .iter()
        .enumerate()
        .filter_map(|(r, word)| {
            word.0.iter().enumerate().find_map(|(c, &ch)| {
                if ch == wildcard_letter {
                    Some((r, c))
                } else {
                    None
                }
            })
        })
        .collect();
    // Now, try calculating the score where wildcard is used in each of those places
    let mut max_score = 0;
    if all_wildcard_indices.is_empty() {
        let mut local_score = 0;
        for (row, word) in board.iter().enumerate() {
            for (col, ch) in word.0.iter().enumerate() {
                local_score += letter_scores.get(ch).unwrap_or(&0) * SCHEMA[row][col];
            }
        }
        // Now add the bonus words score
        let mut new_word: FourWord = ['*'; 4];
        for (i, &(r, c)) in (&BONUS_WORD_INDS).into_iter().enumerate() {
            if let Some(word) = board.get(r) {
                new_word[i] = word.0[c];
            }
        }
        if four_letter_words.contains(&new_word) {
            for (i, &(r, c)) in (&BONUS_WORD_INDS).into_iter().enumerate() {
                local_score += letter_scores.get(&new_word[i]).unwrap_or(&0) * SCHEMA[r][c];
            }
        }
        if max_score < local_score {
            max_score = local_score;
        }
    } else {
        for (row1, col1) in all_wildcard_indices {
            let mut local_score = 0;
            for (row, word) in board.iter().enumerate() {
                for (col, ch) in word.0.iter().enumerate() {
                    if row == row1 && col == col1 {
                        // If this is the wildcard position, use the wildcard letter
                        continue;
                        // score += letter_scores.get(&wildcard_letter).unwrap_or(&0) * SCHEMA[row][col];
                    }
                    local_score += letter_scores.get(ch).unwrap_or(&0) * SCHEMA[row][col];
                }
            }
            // Now add the bonus words score
            let mut new_word: FourWord = ['*'; 4];
            for (i, &(r, c)) in (&BONUS_WORD_INDS).into_iter().enumerate() {
                if let Some(word) = board.get(r) {
                    new_word[i] = word.0[c];
                }
            }
            if four_letter_words.contains(&new_word) {
                for (i, &(r, c)) in (&BONUS_WORD_INDS).into_iter().enumerate() {
                    if r == row1 && c == col1 {
                        // If this is the wildcard position, use the wildcard letter
                        continue;
                    }
                    local_score += letter_scores.get(&new_word[i]).unwrap_or(&0) * SCHEMA[r][c];
                }
            }
            if max_score < local_score {
                max_score = local_score;
            }
        }
    }
    max_score
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
    let file = File::open("sgb-words-med.txt").expect("data.txt not found");
    let bongo_file = File::open("bongo-common-words.txt").expect("data.txt not found");
    // let file = File::open("bongo-common-words.txt").expect("data.txt not found");
    let reader = BufReader::new(file);
    let bongo_reader = BufReader::new(bongo_file);
    let lines = reader.lines().collect::<Vec<_>>();
    let bongo_lines = bongo_reader.lines().collect::<Vec<_>>();
    let lines = lines.into_iter().filter_map(|l| l.ok()).collect::<Vec<_>>();
    println!("Number of lines in file: {}", lines.len());
    let four_words: HashSet<FourWord> = bongo_lines
        .into_iter()
        .filter_map(|l| l.ok())
        .map(|w| w.trim().to_uppercase())
        .filter(|w| w.len() == 4)
        .map(|w| {
            let mut chars = w.chars();
            [
                chars.next().unwrap(),
                chars.next().unwrap(),
                chars.next().unwrap(),
                chars.next().unwrap(),
            ]
        })
        .collect();
    let five_words: HashSet<String> = lines
        .into_iter()
        .map(|w| w.trim().to_uppercase())
        .filter(|w| w.len() == 5)
        .collect();
    println!("Number of 5 words: {}", five_words.len());
    let words_arc = Arc::new(five_words);

    // Generate all possible valid rows
    let valid_words: Vec<Box<ValidWord>> = words_arc
        .par_iter()
        .filter_map(|word| {
            let mut bag = letter_bag.clone();
            let mut wildcard_char: Option<char> = None;
            for c in word.chars() {
                if let Some(pos) = bag.iter().position(|&x| x == c) {
                    bag.remove(pos);
                } else if wildcard_char == None {
                    if let Some(pos) = bag.iter().position(|&x| x == '*') {
                        bag.remove(pos);
                        wildcard_char = Some(c);
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
                    word.chars().next().unwrap(),
                    word.chars().nth(1).unwrap(),
                    word.chars().nth(2).unwrap(),
                    word.chars().nth(3).unwrap(),
                    word.chars().nth(4).unwrap(),
                ],
                wildcard_char,
            )))
        })
        .collect();
    let valid_words_copy: Vec<&ValidWord> = valid_words.iter().map(|b| b.as_ref()).collect();
    print!("Copied {} valid words to memory. ", valid_words.len());
    // flush io
    std::io::stdout().flush().unwrap();

    // Now, since we have all valid words, we can make a collection of all valid combinations of words into 5 rows
    const K: i32 = 5;
    let valid_sets = generate_k_sets(valid_words_copy, K, letter_bag);
    // let valid_sets = generate_k_sets_memo(valid_words_copy.into(), K, 0);
    println!("Total valid sets of {K} rows found: {}", valid_sets.len());
    // Print the first 5 sets
    for (i, set) in valid_sets.iter().take(5).enumerate() {
        println!(
            "Set {}: {:?}",
            i,
            set.iter().map(|w| print_five_word(w.0)).collect::<Vec<_>>()
        );
    }

    // Brute force all possible 5x5 boards (parallel, batched)
    let valid_sets_arc = Arc::new(valid_sets);
    let total = valid_sets_arc.len();
    println!("Total combinations to check: {}", total);
    let batch_size = 100_000usize;
    let num_batches = total.div_ceil(batch_size);
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
                    // Check all permutations of the board
                    let permutation = permute_board(&valid_sets_arc[idx]);
                    for permut in permutation {
                        let score = score_board(&permut, &letter_scores, &four_words);
                        if score > local_best.0 {
                            local_best =
                                (score, permut.iter().map(|word| word.0).collect::<Vec<_>>());
                        }
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
                    std::io::stdout().flush().unwrap();
                }
                local_best
            },
        )
        .max_by_key(|(score, _)| *score)
        .unwrap();
    // for best in bests {
    //     if best.0 > 0 {
    //         println!("\nFound a valid board with score: {}", best.0);
    //         for row in &best.1 {
    //             println!("{}", print_five_word(*row));
    //         }
    //     }
    // }
    println!();
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
