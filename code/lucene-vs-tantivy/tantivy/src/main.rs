// bench index <corpus.tsv> <indexDir>
// bench search <indexDir> <queries.tsv>
use std::{collections::BTreeMap, fs, io::BufRead, path::Path, time::Instant};
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, PhraseQuery, Query, TermQuery};
use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions, STORED};
use tantivy::tokenizer::{LowerCaser, SimpleTokenizer, TextAnalyzer};
use tantivy::{doc, Index, IndexWriter, ReloadPolicy, Term};

fn peak_rss_mb() -> f64 {
    let s = fs::read_to_string("/proc/self/status").unwrap();
    let l = s.lines().find(|l| l.starts_with("VmHWM:")).unwrap();
    l.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse::<f64>().unwrap() / 1024.0
}

fn dir_mb(p: &Path) -> f64 {
    fs::read_dir(p).unwrap().map(|e| e.unwrap().metadata().unwrap().len()).sum::<u64>() as f64 / 1048576.0
}

fn main() {
    let t0 = Instant::now();
    let a: Vec<String> = std::env::args().collect();
    if a[1] == "index" { index(&a[2], &a[3]) } else { search(&a[2], &a[3], t0) }
}

fn index(corpus: &str, dir: &str) {
    let start = Instant::now();
    let mut sb = Schema::builder();
    let id = sb.add_text_field("id", STORED);
    let opts = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default().set_tokenizer("ws").set_index_option(IndexRecordOption::WithFreqsAndPositions));
    let body = sb.add_text_field("body", opts);
    fs::create_dir_all(dir).unwrap();
    let index = Index::create_in_dir(dir, sb.build()).unwrap();
    index.tokenizers().register("ws", TextAnalyzer::builder(SimpleTokenizer::default()).filter(LowerCaser).build());
    let mut w: IndexWriter = index.writer_with_num_threads(1, 256_000_000).unwrap();
    for line in std::io::BufReader::new(fs::File::open(corpus).unwrap()).lines() {
        let line = line.unwrap();
        let (i, text) = line.split_once('\t').unwrap();
        w.add_document(doc!(id => i, body => text)).unwrap();
    }
    w.commit().unwrap();
    let ids = index.searchable_segment_ids().unwrap();
    if ids.len() > 1 { w.merge(&ids).wait().unwrap(); }
    w.wait_merging_threads().unwrap();
    println!("index_seconds={:.1} index_mb={:.1} peak_rss_mb={:.0}",
        start.elapsed().as_secs_f64(), dir_mb(Path::new(dir)), peak_rss_mb());
}

fn search(dir: &str, queries: &str, t0: Instant) {
    let index = Index::open_in_dir(dir).unwrap();
    index.tokenizers().register("ws", TextAnalyzer::builder(SimpleTokenizer::default()).filter(LowerCaser).build());
    let body = index.schema().get_field("body").unwrap();
    let reader = index.reader_builder().reload_policy(ReloadPolicy::Manual).try_into().unwrap();
    let s = reader.searcher();
    let qs: Vec<(String, Vec<String>)> = fs::read_to_string(queries).unwrap().lines()
        .map(|l| { let (t, q) = l.split_once('\t').unwrap(); (t.to_string(), q.split(' ').map(String::from).collect()) })
        .collect();
    let parse = |t: &str, terms: &[String]| -> Box<dyn Query> {
        let tm: Vec<Term> = terms.iter().map(|x| Term::from_field_text(body, x)).collect();
        match t {
            "term" => Box::new(TermQuery::new(tm[0].clone(), IndexRecordOption::WithFreqs)),
            "phrase" => Box::new(PhraseQuery::new(tm)),
            _ => {
                let o = if t == "and" { Occur::Must } else { Occur::Should };
                Box::new(BooleanQuery::new(tm.into_iter().map(|x| {
                    (o, Box::new(TermQuery::new(x, IndexRecordOption::WithFreqs)) as Box<dyn Query>)
                }).collect()))
            }
        }
    };
    if std::env::var("CHECK").is_ok() {
        for (t, terms) in &qs {
            println!("{}\t{}\t{}", t, terms.join(" "), s.search(&*parse(t, terms), &tantivy::collector::Count).unwrap());
        }
        return;
    }
    let top10 = TopDocs::with_limit(10).order_by_score();
    s.search(&*parse(&qs[0].0, &qs[0].1), &top10).unwrap();
    let first_ms = t0.elapsed().as_secs_f64() * 1e3;
    let mut best = vec![u128::MAX; qs.len()];
    for round in 0..70 {                                     // rounds 0-49 are warm-up
        for (i, (t, terms)) in qs.iter().enumerate() {
            let q = parse(t, terms);
            let st = Instant::now();
            s.search(&*q, &top10).unwrap();
            let dt = st.elapsed().as_nanos();
            if round >= 50 { best[i] = best[i].min(dt); }
        }
    }
    let mut by: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for (i, (t, _)) in qs.iter().enumerate() { by.entry(t).or_default().push(best[i] as f64 / 1e3); }
    let mut out = format!("first_query_ms_after_start={:.0}", first_ms);
    for (k, mut v) in by { v.sort_by(|a, b| a.partial_cmp(b).unwrap()); out += &format!(" {}_median_us={:.0}", k, v[v.len() / 2]); }
    println!("{} peak_rss_mb={:.0}", out, peak_rss_mb());
}
