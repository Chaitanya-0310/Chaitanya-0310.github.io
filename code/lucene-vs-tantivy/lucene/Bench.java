import java.nio.file.*;
import java.util.*;
import org.apache.lucene.analysis.standard.StandardAnalyzer;
import org.apache.lucene.document.*;
import org.apache.lucene.index.*;
import org.apache.lucene.search.*;
import org.apache.lucene.store.*;

// java Bench index <corpus.tsv> <indexDir>
// java Bench search <indexDir> <queries.tsv>
public class Bench {
    static final long T0 = System.nanoTime();

    static long peakRssKb() throws Exception {
        for (String l : Files.readAllLines(Path.of("/proc/self/status")))
            if (l.startsWith("VmHWM:")) return Long.parseLong(l.replaceAll("\\D", ""));
        return -1;
    }

    static long dirBytes(Path p) throws Exception {
        try (var s = Files.walk(p)) { return s.filter(Files::isRegularFile).mapToLong(f -> f.toFile().length()).sum(); }
    }

    public static void main(String[] a) throws Exception {
        if (a[0].equals("index")) index(Path.of(a[1]), Path.of(a[2])); else search(Path.of(a[1]), Path.of(a[2]));
    }

    static void index(Path corpus, Path dir) throws Exception {
        long start = System.nanoTime();
        IndexWriterConfig cfg = new IndexWriterConfig(new StandardAnalyzer()).setRAMBufferSizeMB(256);
        try (IndexWriter w = new IndexWriter(FSDirectory.open(dir), cfg);
             var lines = Files.lines(corpus)) {
            lines.forEach(line -> {
                int tab = line.indexOf('\t');
                Document d = new Document();
                d.add(new StoredField("id", line.substring(0, tab)));
                d.add(new TextField("body", line.substring(tab + 1), Field.Store.NO));
                try { w.addDocument(d); } catch (Exception e) { throw new RuntimeException(e); }
            });
            w.forceMerge(1);
            w.commit();
        }
        System.out.printf("index_seconds=%.1f index_mb=%.1f peak_rss_mb=%.0f%n",
            (System.nanoTime() - start) / 1e9, dirBytes(dir) / 1048576.0, peakRssKb() / 1024.0);
    }

    static Query parse(String type, String[] t) {
        switch (type) {
            case "term": return new TermQuery(new Term("body", t[0]));
            case "phrase": return new PhraseQuery("body", t);
            default:
                BooleanQuery.Builder b = new BooleanQuery.Builder();
                BooleanClause.Occur o = type.equals("and") ? BooleanClause.Occur.MUST : BooleanClause.Occur.SHOULD;
                for (String s : t) b.add(new TermQuery(new Term("body", s)), o);
                return b.build();
        }
    }

    static void search(Path dir, Path queriesFile) throws Exception {
        List<String[]> qs = new ArrayList<>();
        for (String l : Files.readAllLines(queriesFile)) qs.add(l.split("\t"));
        try (DirectoryReader r = DirectoryReader.open(FSDirectory.open(dir))) {
            IndexSearcher s = new IndexSearcher(r);
            s.setQueryCache(null);
            if (System.getenv("CHECK") != null) {
                for (String[] q : qs) System.out.println(q[0] + "\t" + q[1] + "\t" + s.count(parse(q[0], q[1].split(" "))));
                return;
            }
            String[] q0 = qs.get(0);
            s.search(parse(q0[0], q0[1].split(" ")), 10);
            double firstMs = (System.nanoTime() - T0) / 1e6;
            Map<String, List<Double>> best = new TreeMap<>();
            long[] b = new long[qs.size()];
            Arrays.fill(b, Long.MAX_VALUE);
            for (int round = 0; round < 70; round++) {          // rounds 0-49 are warm-up
                for (int i = 0; i < qs.size(); i++) {
                    Query q = parse(qs.get(i)[0], qs.get(i)[1].split(" "));
                    long t = System.nanoTime();
                    s.search(q, 10);
                    long dt = System.nanoTime() - t;
                    if (round >= 50) b[i] = Math.min(b[i], dt);
                }
            }
            for (int i = 0; i < qs.size(); i++) best.computeIfAbsent(qs.get(i)[0], k -> new ArrayList<>()).add(b[i] / 1e3);
            StringBuilder out = new StringBuilder(String.format("first_query_ms_after_start=%.0f", firstMs));
            for (var e : best.entrySet()) {
                List<Double> v = e.getValue(); Collections.sort(v);
                out.append(String.format(" %s_median_us=%.0f", e.getKey(), v.get(v.size() / 2)));
            }
            System.out.println(out + String.format(" peak_rss_mb=%.0f", peakRssKb() / 1024.0));
        }
    }
}
