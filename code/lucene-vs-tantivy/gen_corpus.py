"""Generate a deterministic synthetic corpus and query set.

Words follow a Zipf distribution, like real text, so a few words are very
common and most are rare. Output:
  data/corpus.tsv   id<TAB>text, one document per line
  data/queries.tsv  type<TAB>terms   (type is term, and, or, phrase)
"""
import itertools, os, random

N_DOCS, VOCAB, WORDS_PER_DOC = 500_000, 50_000, 60
rng = random.Random(42)
syll = ["ka", "lo", "mi", "ra", "te", "su", "no", "vi", "ze", "po", "an", "el", "or", "ul", "is"]
vocab = set()
while len(vocab) < VOCAB:
    vocab.add("".join(rng.choice(syll) for _ in range(rng.randint(2, 4))))
vocab = sorted(vocab); rng.shuffle(vocab)
cum = list(itertools.accumulate(1 / (r + 1) for r in range(VOCAB)))
os.makedirs("data", exist_ok=True)
docs = []
with open("data/corpus.tsv", "w") as f:
    for i in range(N_DOCS):
        words = rng.choices(vocab, cum_weights=cum, k=WORDS_PER_DOC)
        if i < 2000:
            docs.append(words)
        f.write(f"{i}\t{' '.join(words)}\n")
with open("data/queries.tsv", "w") as f:
    for _ in range(100):
        f.write(f"term\t{rng.choice(vocab[50:5000])}\n")
    for _ in range(100):
        f.write(f"and\t{rng.choice(vocab[:200])} {rng.choice(vocab[200:5000])}\n")
    for _ in range(100):
        f.write(f"or\t{rng.choice(vocab[:200])} {rng.choice(vocab[200:5000])}\n")
    for _ in range(100):  # phrases taken from real documents, so they match
        d = rng.choice(docs); j = rng.randrange(len(d) - 1)
        f.write(f"phrase\t{d[j]} {d[j + 1]}\n")
print("wrote data/corpus.tsv and data/queries.tsv")
