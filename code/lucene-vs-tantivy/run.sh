#!/bin/sh
# Builds and runs both engines in Docker, each limited to the same CPUs and memory.
# Usage: ./run.sh            (run from this folder, after python3 gen_corpus.py)
set -e
cd "$(dirname "$0")"
LUCENE=10.5.1
LIMITS="--cpus=2 --memory=4g"
mkdir -p lucene/lib out

[ -f lucene/lib/lucene-core-$LUCENE.jar ] || curl -sfL -o lucene/lib/lucene-core-$LUCENE.jar \
  https://repo1.maven.org/maven2/org/apache/lucene/lucene-core/$LUCENE/lucene-core-$LUCENE.jar

echo "== building"
docker run --rm -v "$PWD/lucene":/b -w /b eclipse-temurin:21 javac -cp lib/lucene-core-$LUCENE.jar -d classes Bench.java
docker run --rm -v "$PWD/tantivy":/b -v bench-cargo:/usr/local/cargo/registry -w /b rust:1 cargo build --release -q

J() { docker run --rm $LIMITS -v "$PWD":/w -w /w eclipse-temurin:21 \
        java --add-modules jdk.incubator.vector --enable-native-access=ALL-UNNAMED -cp lucene/classes:lucene/lib/lucene-core-$LUCENE.jar Bench "$@"; }
T() { docker run --rm $LIMITS -v "$PWD":/w -w /w rust:1 tantivy/target/release/bench "$@"; }

rm -rf out/lucene-index out/tantivy-index
echo "== indexing";  printf 'lucene  '; J index data/corpus.tsv out/lucene-index
                     printf 'tantivy '; T index data/corpus.tsv out/tantivy-index
for i in 1 2 3; do
  echo "== search run $i"
  printf 'lucene  '; J search out/lucene-index data/queries.tsv
  printf 'tantivy '; T search out/tantivy-index data/queries.tsv
done
