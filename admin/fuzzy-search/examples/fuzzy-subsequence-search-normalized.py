def is_subsequence(query, candidate):
    q_len, c_len = len(query), len(candidate)
    q_idx, c_idx = 0, 0
    match_indices = []

    while q_idx < q_len and c_idx < c_len:
        if query[q_idx] == candidate[c_idx]:
            match_indices.append(c_idx)
            q_idx += 1
        c_idx += 1

    if q_idx == q_len:
        return match_indices
    else:
        return None

def score_match_normalized(match_indices, candidate, max_len):
    span = match_indices[-1] - match_indices[0] + 1
    span_norm = span / len(candidate)
    length_norm = len(candidate) / max_len
    score = 0.7 * span_norm + 0.3 * length_norm
    return score

def fuzzy_subsequence_search(query, candidates):
    max_len = max(len(c) for c in candidates)
    results = []
    for candidate in candidates:
        match_indices = is_subsequence(query, candidate)
        if match_indices:
            score = score_match_normalized(match_indices, candidate, max_len)
            results.append((score, candidate))
    results.sort(key=lambda x: x[0])
    return [candidate for _, candidate in results]

# Example usage:
candidates = [
    "openai/gpt-4o-2024-08-06",
    "openai/gpt-4o-mini-2024-07-18",
    "openai/gpt-4.1-mini-2025-04-14",
    "openai/gpt-4.1",
    "openai/gpt-5-mini",
    "openai/gpt-5-nano",
    "openai/gpt-5"
]

print(fuzzy_subsequence_search("g4", candidates))
print(fuzzy_subsequence_search("ogp5", candidates))