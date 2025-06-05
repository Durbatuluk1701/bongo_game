"""
Read in data.txt, this is you set of valid word.

You then must utilize a global variable POSSIBLE_LETTERS = { char : character, num : int, score : int }[]
Note: these is a special character called ANY, it can be any character you want, you always get only 1 of them, and its score is always 0


You will then get a schema of how to enter these letters into 5 lines with 5 characters per line, the schema will be of form: [ mult1, mult2, mult3, mult4, mult5 ] * 5.

The score for each line is its char value times the multiplier per char.score at the correct place

Generate a solution to find the highest score based off of an exhaustive brute-force search. Although words must always be valid
"""

# Example hardcoded letter bag and schema
from functools import reduce


POSSIBLE_LETTERS = [
    {"char": "G", "num": 1, "score": 45},
    {"char": "B", "num": 3, "score": 50},
    {"char": "M", "num": 1, "score": 35},
    {"char": "D", "num": 1, "score": 30},
    {"char": "N", "num": 2, "score": 20},
    {"char": "U", "num": 1, "score": 15},
    {"char": "L", "num": 1, "score": 9},
    {"char": "T", "num": 2, "score": 10},
    {"char": "O", "num": 2, "score": 7},
    {"char": "R", "num": 2, "score": 7},
    {"char": "S", "num": 3, "score": 5},
    # {"char": "ANY", "num": 1, "score": 0},
    {"char": "A", "num": 4, "score": 5},
    {"char": "E", "num": 2, "score": 5},
]

assert (
    reduce(lambda x, y: x + y, map(lambda val: val["num"], POSSIBLE_LETTERS), 0) == 25
)

SCHEMA = [
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [1, 2, 1, 3, 1],
    [1, 1, 1, 2, 1],
]

import itertools
from collections import Counter
import concurrent.futures


def load_words(filename):
    with open(filename) as f:
        return set(line.strip().upper() for line in f if len(line.strip()) == 5)


WORDS = load_words("data.txt")

# Build the letter bag
letter_bag = []
letter_scores = {}
for entry in POSSIBLE_LETTERS:
    letter_scores[entry["char"]] = entry["score"]
    # Ensure num is int
    num = int(entry["num"])
    letter_bag.extend([entry["char"]] * num)


# Helper to generate all possible 5-letter words from the bag (with ANY as wildcard)
def possible_words(bag, words):
    bag_counter = Counter(bag)
    results = []
    for word in words:
        wc_used = False
        temp = bag_counter.copy()
        for c in word:
            if temp[c] > 0:
                temp[c] -= 1
            elif temp["ANY"] > 0 and not wc_used:
                temp["ANY"] -= 1
                wc_used = True
            else:
                break
        else:
            results.append((word, wc_used))
    return results


# Generate all possible valid rows
valid_rows = possible_words(letter_bag, WORDS)

print("Total valid rows found:", len(valid_rows))
print(f"Row {range(0, 5)} examples:", [word for word, _ in valid_rows[:5]])
if len(valid_rows) > 5:
    print("... and more rows available.")

# Brute force all possible 5x5 boards (very slow for large bags!)
# For demo, we only try combinations of valid rows
def board_score_and_valid(board_rows):
    used = Counter()
    any_used = 0
    for word, wc in board_rows:
        for c in word:
            used[c] += 1
        if wc:
            any_used += 1
    for entry in POSSIBLE_LETTERS:
        c = entry["char"]
        num = int(entry["num"])
        if used[c] > num:
            return None
    if any_used > 1:
        return None
    # Check columns are valid words
    cols = ["".join(word for word, _ in board_rows)[i] for i in range(5)]
    if not all(col in WORDS for col in cols):
        return None
    # Score the board
    score = 0
    for r, (word, _) in enumerate(board_rows):
        for c, ch in enumerate(word):
            score += letter_scores.get(ch, 0) * SCHEMA[r][c]
    return (score, [word for word, _ in board_rows])

print("Starting parallel brute-force search...")
max_score = 0
best_board = None
with concurrent.futures.ProcessPoolExecutor() as executor:
    chunk_size = 10000
    combos = itertools.product(valid_rows, repeat=5)
    futures = []
    chunk = []
    for i, board_rows in enumerate(combos):
        chunk.append(board_rows)
        if len(chunk) == chunk_size:
            futures.append(executor.submit(
                lambda boards: max(filter(None, map(board_score_and_valid, boards)), default=(0, None)),
                chunk))
            chunk = []
    if chunk:
        futures.append(executor.submit(
            lambda boards: max(filter(None, map(board_score_and_valid, boards)), default=(0, None)),
            chunk))
    for fut in concurrent.futures.as_completed(futures):
        score, board = fut.result()
        if score > max_score:
            max_score = score
            best_board = board

if best_board:
    print("Best board:")
    for row in best_board:
        print(row)
    print("Score:", max_score)
else:
    print("No valid board found.")
