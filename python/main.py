import sys
import random
import string
import time
import os

def generate_random_words(num_words):
    return [''.join(random.choices(string.ascii_letters + string.digits, k=random.randint(3, 20))) for _ in range(num_words)]

def insert_word_if_unique(words, word):
    if word not in words:
        words.append(word)
        return True
    return False

def remove_word_if_present(words, word):
    if word in words:
        words.remove(word)
        return True
    return False

def write_words_to_file(words, file_path):
    with open(file_path, 'w') as file:
        for word in words:
            file.write(f"{word}\n")

def read_words_from_file(file_path):
    with open(file_path, 'r') as file:
        return [line.strip() for line in file]

def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <file> <num>")
        sys.exit(1)

    file_path = sys.argv[1]
    num_words = int(sys.argv[2])
    insert_remove_words_count = 100

    # truncate file if it exists
    if os.path.exists(file_path):
        os.open(file_path, os.O_TRUNC)

    # Generate random words
    start = time.perf_counter()
    words = generate_random_words(num_words)
    gen_time = time.perf_counter() - start
    print(f"Creating random list of words... took {(gen_time * 1000):.6f} ms")
    assert len(words) == num_words
    # input("Press enter to continue...")

    # Sort words
    start = time.perf_counter()
    words.sort()
    sort_time = time.perf_counter() - start
    print(f"Sorting words... took {(sort_time * 1000):.6f} ms")
    # input("Press enter to continue...")

    # Remove duplicates
    start = time.perf_counter()
    words = list(dict.fromkeys(words))
    dedup_time = time.perf_counter() - start
    words_len = len(words)
    print(f"Removing duplicates... took {(dedup_time * 1000):.6f} ms")
    # input("Press enter to continue...")

    # Write words to file
    start = time.perf_counter()
    write_words_to_file(words, file_path)
    write_time = time.perf_counter() - start
    print(f"Writing words to file... took {(write_time * 1000):.6f} ms")
    # input("Press enter to continue...")

    # Read words from file
    start = time.perf_counter()
    words = read_words_from_file(file_path)
    read_time = time.perf_counter() - start
    print(f"Reading words from file... took {(read_time * 1000):.6f} ms")
    # input("Press enter to continue...")
    assert len(words) == words_len

    # Get first insert_remove_words_count words
    words_to_remove = words[:insert_remove_words_count]

    # Insert words while present 1
    start = time.perf_counter()
    for word in words_to_remove:
        insert_word_if_unique(words, word)
    insert_present_time1 = time.perf_counter() - start
    print(f"Inserting {insert_remove_words_count} words while present 1 ... took {(insert_present_time1 * 1000):.6f} ms")
    # input("Press enter to continue...")
    assert len(words) == words_len

    # Remove words 2
    start = time.perf_counter()
    for word in words_to_remove:
        remove_word_if_present(words, word)
    remove_time2 = time.perf_counter() - start
    print(f"Removing {insert_remove_words_count} words 2 ... took {(remove_time2 * 1000):.6f} ms")
    # input("Press enter to continue...")
    assert len(words) == words_len - insert_remove_words_count

    # Insert words back in 2
    start = time.perf_counter()
    for word in words_to_remove:
        insert_word_if_unique(words, word)
    insert_not_present_time1 = time.perf_counter() - start
    print(f"Inserting {insert_remove_words_count} words back in 1 ... took {(insert_not_present_time1 * 1000):.6f} ms")
    # input("Press enter to continue...")
    assert len(words) == words_len

    # Reverse words
    start = time.perf_counter()
    words.reverse()
    reverse_time = time.perf_counter() - start
    print(f"Reversing words... took {(reverse_time * 1000):.6f} ms")
    # input("Press enter to continue...")
    assert len(words) == words_len

    # Write words back to file
    start = time.perf_counter()
    write_words_to_file(words, file_path)
    write_time_sorted = time.perf_counter() - start
    print(f"Writing sorted words back to file... took {(write_time_sorted * 1000):.6f} ms")

    print(f"Total time to complete test: {((read_time + sort_time + dedup_time + reverse_time + write_time_sorted) * 1000):.6f} ms")

if __name__ == "__main__":
    main()