#
#  This test proves with a simple test that the get_close_matches function
#  from the python difflib module works virtually identically to a simple
#  implementation of the function from the fuzzy-matcher crate, when the
#  cutoff value is divided in half for the `similar` function.
#
#  It also compares both functions with the jaro_winkler_metric function, which
#  appears to be superior for the spelling correction use case.
#

from difflib import get_close_matches
from jaro import jaro_winkler_metric
import argparse

def similar(a: str, b: str) -> float:
    a_len = len(a)
    b_len = len(b)
    matches = 0

    # break string a into a list of characters
    a_chars = [c for c in a]
    b_chars = [c for c in b]

    intersection = set(a_chars).intersection(b_chars)
    for c in intersection:
        matches += min(a.count(c), b.count(c))

    if a_len == 0 or b_len == 0:
        return 0.0

    return float(matches) / (a_len + b_len)

def get_similar_words(word, possibilities, n=3, cutoff=0.6):
    cutoff /= 2.0
    matches = []
    for possibility in possibilities:
        score = similar(word, possibility)
        if score >= cutoff:
            matches.append((score, possibility))
    matches.sort(reverse=True)
    return [m for (score, m) in matches[:n]]

def get_jaro_winkler_words(word, possibilities, n=3, cutoff=0.6):
    matches = []
    for possibility in possibilities:
        score = jaro_winkler_metric(word, possibility)
        if score >= cutoff:
            matches.append((score, possibility))
    matches.sort(reverse=True)
    return [m for (score, m) in matches[:n]]

def main():
    parser = argparse.ArgumentParser(description="get_close_matches test")
    parser.add_argument("-c", "--cutoff", help="floating point cutoff value", type=float, default=0.6)
    # parser.add_argument("arg2", help="The second argument")
    parser.add_argument("-s", "--string", help="optional string", default="apple")
    parser.add_argument("-a", dest="alt", action="store_true", help="alternate similar function")
    parser.add_argument("-j", dest="jaro", action="store_true", help="jaro-winkler function")
    parser.add_argument("-n", dest="n", help="number of matches", type=int, default=10)
    args = parser.parse_args()

    # print("Argument 1:", args.arg1)
    # print("Argument 2:", args.arg2)
    # print("Optional argument:", args.optional)

    possibilities = [
        "ample",
        "amplification",
        "apple",
        "apply",
        "application",
        "applicable",
        "applicability",
        "appointment",
        "apricot",
        "banana",
        "blueberry",
        "cherry",
        "grape",
        "grapple",
        "grappling",
        "orange",
        "peach",
        "plum",
        "strawberry",
    ]

    test_string = args.string
    print(f"Test string: {test_string}")
    print(f"Cutoff: {args.cutoff}")
    print("Matches:")
    if args.alt:
        matches = get_similar_words(test_string, possibilities, args.n, args.cutoff)
    elif args.jaro:
        matches = get_jaro_winkler_words(test_string, possibilities, args.n, args.cutoff)
    else:
        matches = get_close_matches(test_string, possibilities, args.n, args.cutoff)
    matches = matches[:args.n]
    for match in matches:
        print(f"  {match}")

if __name__ == "__main__":
    main()
