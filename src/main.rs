use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
struct Letter {
    ch: char,
    num: usize,
    //    score: i32,
}

const POSSIBLE_LETTERS: [Letter; 14] = [
    Letter { ch: 'G', num: 1 },
    Letter { ch: 'B', num: 3 },
    Letter { ch: 'M', num: 1 },
    Letter { ch: 'D', num: 1 },
    Letter { ch: 'N', num: 2 },
    Letter { ch: 'U', num: 1 },
    Letter { ch: 'L', num: 1 },
    Letter { ch: 'T', num: 2 },
    Letter { ch: 'O', num: 2 },
    Letter { ch: 'R', num: 2 },
    Letter { ch: 'S', num: 3 },
    Letter { ch: 'A', num: 4 },
    Letter { ch: 'E', num: 2 },
    Letter {
        ch: '*',
        num: 1, // Wildcard
    },
];

fn letter_to_score(c: &char) -> u32 {
    match c {
        'B' => 50,
        'G' => 45,
        'M' => 35,
        'D' => 30,
        'N' => 20,
        'U' => 15,
        'T' => 10,
        'L' => 9,
        'O' => 7,
        'R' => 7,
        'S' => 5,
        'A' => 5,
        'E' => 5,
        '*' => 0,
        _ => 0,
    }
}

const SCHEMA: [[u32; 5]; 5] = [
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [1, 2, 1, 3, 1],
    [1, 1, 1, 2, 1],
];

const BONUS_WORD_INDS: [(usize, usize); 4] = [(0, 2), (1, 2), (2, 2), (3, 3)];

// const POSSIBLE_LETTERS: [Letter; 13] = [
//     Letter {
//         ch: 'B',
//         num: 2,
//         score: 50,
//     },
//     Letter {
//         ch: 'H',
//         num: 1,
//         score: 40,
//     },
//     Letter {
//         ch: 'M',
//         num: 2,
//         score: 40,
//     },
//     Letter {
//         ch: 'C',
//         num: 1,
//         score: 35,
//     },
//     Letter {
//         ch: 'Y',
//         num: 1,
//         score: 35,
//     },
//     Letter {
//         ch: 'L',
//         num: 2,
//         score: 10,
//     },
//     Letter {
//         ch: 'I',
//         num: 1,
//         score: 9,
//     },
//     Letter {
//         ch: 'T',
//         num: 3,
//         score: 9,
//     },
//     Letter {
//         ch: 'R',
//         num: 1,
//         score: 7,
//     },
//     Letter {
//         ch: 'A',
//         num: 1,
//         score: 5,
//     },
//     Letter {
//         ch: 'S',
//         num: 3,
//         score: 5,
//     },
//     Letter {
//         ch: 'E',
//         num: 6,
//         score: 5,
//     },
//     Letter {
//         ch: '*',
//         num: 1,   // Wildcard
//         score: 0, // Wildcard has no score
// ];

// const SCHEMA: [[i32; 5]; 5] = [
//     [1, 1, 2, 1, 1],
//     [1, 1, 1, 1, 1],
//     [1, 2, 1, 1, 1],
//     [1, 1, 1, 1, 3],
//     [1, 1, 1, 1, 1],
// ];

// const BONUS_WORD_INDS: [(usize, usize); 4] = [(0, 1), (1, 2), (2, 3), (3, 3)];

type ValidWord = (String, Option<char>); // (word, wildcard_used)

fn prescore_word_in_row(row: usize, word: &ValidWord) -> u32 {
    let mut best_score = 0;

    if let Some(wildcard_char) = word.1 {
        if SCHEMA[row].iter().any(|&x| x != 1) {
            for col in word
                .0
                .char_indices()
                .filter(|(_, c)| *c == wildcard_char)
                .map(|(i, _)| i)
            {
                let new_score = score_word(4, word, Some((4, col)));
                if new_score > best_score {
                    best_score = new_score;
                }
            }
        } else {
            let col = word.0.find(wildcard_char).unwrap();
            best_score = score_word(row, word, Some((row, col)))
        }
    } else {
        best_score = score_word(row, word, None);
    }
    best_score
}

fn score_word(row: usize, word: &ValidWord, wildcard_index: Option<(usize, usize)>) -> u32 {
    println!("scoring word {:?}", word);
    let mut word_score = 0.0;

    for (col, ch) in word.0.chars().enumerate() {
        if wildcard_index == Some((row, col)) {
            // If this is the wildcard position, use the wildcard letter
            continue;
            // score += letter_scores.get(&wildcard_letter).unwrap_or(&0) * SCHEMA[row][col];
        }
        word_score += (letter_to_score(&ch) * SCHEMA[row][col]) as f64;
    }
    if true
    //is common word
    {
        f64::ceil(word_score * 1.3) as u32
    } else {
        f64::ceil(word_score) as u32
    }
}

fn score_board(board: &Vec<Option<&ValidWord>>, bonus_word_used: bool) -> u32 {
    let mut wildcard_letter = '*';
    for word in board.iter() {
        if let Some(wildchar) = word.unwrap().1 {
            // If wildcard is used, we can use any letter in its place
            wildcard_letter = wildchar;
        }
    }
    // Now, find all places that letter is used in this board
    let all_wildcard_indices: Vec<(usize, usize)> = board
        .iter()
        .enumerate()
        .filter_map(|(r, word)| {
            word.unwrap().0.chars().enumerate().find_map(|(c, ch)| {
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
            local_score += score_word(row, word.unwrap(), None);
        }
        // Now add the bonus words score
        let mut new_word = ['*'; 4];
        for (i, &(r, c)) in BONUS_WORD_INDS.iter().enumerate() {
            if let Some(word) = board.get(r) {
                new_word[i] = word.unwrap().0.chars().nth(c).unwrap();
            }
        }
        if bonus_word_used {
            let mut word_score = 0.0;
            for (i, &(r, c)) in BONUS_WORD_INDS.iter().enumerate() {
                word_score += (letter_to_score(&new_word[i]) * SCHEMA[r][c]) as f64;
            }
            if true
            //is common word
            {
                local_score += f64::ceil(word_score * 1.3) as u32;
            }
        }
        if max_score < local_score {
            max_score = local_score;
        }
    } else {
        for (row1, col1) in all_wildcard_indices {
            let mut local_score = 0;
            for (row, word) in board.iter().enumerate() {
                local_score += score_word(row, word.unwrap(), Some((row1, col1)));
            }
            // Now add the bonus words score
            let mut new_word = ['*'; 4];
            for (i, &(r, c)) in BONUS_WORD_INDS.iter().enumerate() {
                if let Some(word) = board.get(r) {
                    new_word[i] = word.unwrap().0.chars().nth(c).unwrap();
                }
            }
            if bonus_word_used {
                let mut word_score = 0.0;
                for (i, &(r, c)) in BONUS_WORD_INDS.iter().enumerate() {
                    if r == row1 && c == col1 {
                        // If this is the wildcard position, use the wildcard letter
                        continue;
                    }
                    word_score += (letter_to_score(&new_word[i]) * SCHEMA[r][c]) as f64;
                }
                if true
                //is common word
                {
                    local_score += f64::ceil(word_score * 1.3) as u32;
                }
            }
            if max_score < local_score {
                max_score = local_score;
            }
        }
    }
    max_score
}

fn generate_boards_from_bonus<'a>(
    bonus_word: &ValidWord,
    valid_words: Vec<&'a ValidWord>,
    letter_bag: Vec<char>,
    row: usize,
) -> Vec<Vec<Option<&'a ValidWord>>> {
    if row > 4 {
        return vec![vec![None; 5]];
    }
    let n = valid_words.len();
    if BONUS_WORD_INDS.map(|(i, _)| i).contains(&row) {
        let index = BONUS_WORD_INDS[row].1;

        (0..n)
            .into_iter()
            .map(|i| {
                let cur_valid_word = &valid_words[i];
                if cur_valid_word.0.chars().nth(index) != bonus_word.0.chars().nth(row) {
                    return vec![];
                }

                // Prune word bag
                let mut new_letter_bag = letter_bag.clone();
                for c in cur_valid_word.0.chars() {
                    if let Some(pos) = new_letter_bag.iter().position(|&x| x == c) {
                        new_letter_bag.remove(pos);
                    } else if let Some(_ind) = cur_valid_word.1 {
                        // If wildcard is used, remove it
                        if let Some(pos) = new_letter_bag.iter().position(|&x| x == '*') {
                            new_letter_bag.remove(pos);
                        } else {
                            return vec![];
                        }
                    } else {
                        return vec![];
                    }
                }
                // Drop off other valid_words that are not valid for the current word_bag
                let next_valid_words = valid_words
                    .iter()
                    .filter(|&&w| {
                        w.0.chars().all(|c| {
                            new_letter_bag
                                .iter()
                                .any(|&x| x == c || (w.1 == Some(c) && x == '*'))
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                generate_boards_from_bonus(bonus_word, next_valid_words, new_letter_bag, row + 1)
                    .into_iter()
                    .map(|mut set| {
                        //set.push(cur_valid_word);
                        set[row] = Some(cur_valid_word);
                        set
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect()
    } else {
        (0..n)
            .into_iter()
            .map(|i| {
                let cur_valid_word = &valid_words[i];
                // Prune word bag
                let mut new_word_bag = letter_bag.clone();
                for c in cur_valid_word.0.chars() {
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
                let next_valid_words = valid_words
                    .iter()
                    .filter(|&&w| {
                        w.0.chars().all(|c| {
                            new_word_bag
                                .iter()
                                .any(|&x| x == c || (w.1 == Some(c) && x == '*'))
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                generate_boards_from_bonus(bonus_word, next_valid_words, new_word_bag, row + 1)
                    .into_iter()
                    .map(|mut set| {
                        //set.push(cur_valid_word);
                        set[row] = Some(cur_valid_word);
                        set
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect()
    }
}

fn main() {
    // Hardcoded letter bag
    let mut letter_bag = Vec::new();
    for l in &POSSIBLE_LETTERS {
        for _ in 0..l.num {
            letter_bag.push(l.ch);
        }
    }

    // Read words from file
    let file = File::open("bongo-common-words.txt").expect("data.txt not found");
    let reader = BufReader::new(file);
    let lines = reader.lines().collect::<Vec<_>>();
    let lines = lines.into_iter().filter_map(|l| l.ok()).collect::<Vec<_>>();
    println!("Number of lines in file: {}", lines.len());
    // Generate all possible valid rows
    let words_arc = Arc::new(lines);
    let valid_words: Vec<ValidWord> = words_arc
        .par_iter()
        .filter_map(|word| {
            let mut bag = letter_bag.clone();
            let mut wildcard_char: Option<char> = None;
            for c in word.chars() {
                if let Some(pos) = bag.iter().position(|&x| x == c) {
                    bag.remove(pos);
                } else if wildcard_char.is_none() {
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
            Some((word.to_ascii_uppercase(), wildcard_char))
        })
        .collect();

    let bonus_words: Vec<&ValidWord> = valid_words
        .iter()
        .filter(|w| w.0.len() == BONUS_WORD_INDS.len())
        .collect();
    println!("Number of bonus words: {}", bonus_words.len());

    // TODO: adjust to check 3 and 4 words as well
    let mut valid_words: Vec<&ValidWord> = valid_words.iter().filter(|w| w.0.len() == 5).collect();
    println!("Number of 5 words: {}", valid_words.len());
    // flush io
    std::io::stdout().flush().unwrap();

    valid_words.sort_by(|&x, &y| prescore_word_in_row(4, x).cmp(&prescore_word_in_row(4, y)));
    println!("{:?}", valid_words);

    let progress = Arc::new(Mutex::new(0usize));
    let total_bonus = bonus_words.len();
    let scored_sets = vec![("BOMB".to_string(), None)] //bonus_words
        .par_iter()
        .map_init(
            || progress.clone(),
            |progress, bonus_word| {
                let result = generate_boards_from_bonus(
                    bonus_word,
                    valid_words.clone(),
                    letter_bag.clone(),
                    0,
                );

                let result = result
                    .iter()
                    .fold((vec![], 0), |(prev_board, prev_score), board| {
                        let score = score_board(board, true);
                        if score > prev_score {
                            (board.to_vec(), score)
                        } else {
                            (prev_board, prev_score)
                        }
                    });

                // Progress bar update
                {
                    let mut done = progress.lock().unwrap();
                    *done += 1;
                    let percent = (*done as f64) * 100.0 / (total_bonus as f64);
                    let bar_len = 40;
                    let filled = (percent / 100.0 * bar_len as f64).round() as usize;
                    let bar: String = "#".repeat(filled) + &"-".repeat(bar_len - filled);
                    print!("\r[{}] {:.2}% ({} / {})", bar, percent, *done, total_bonus);
                    std::io::stdout().flush().unwrap();
                }

                result
            },
        )
        .collect::<Vec<(Vec<Option<&ValidWord>>, _)>>();
    // Print the first 5 sets
    for (i, set) in scored_sets.iter().take(5).enumerate() {
        println!(
            "Set {} with score {}: {:?}",
            i,
            set.1,
            set.0
                .iter()
                .map(|w| w.unwrap().0.clone())
                .collect::<Vec<_>>(),
        );
    }

    let best = scored_sets
        .into_iter()
        .max_by_key(|(_, score)| *score)
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
    if best.1 > 0 {
        println!("Best board:");
        for row in &best.0 {
            println!("{:?}", row);
        }
        println!("Score: {}", best.1);
    } else {
        println!("No valid board found.");
    }
}
