//! Benchmarks for fi-accel-core
//!
//! Run with: cargo bench -p fi-accel-core

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use fi_accel_core::{embedding, diff, log, json};

#[cfg(feature = "chunker")]
use fi_accel_core::chunker;

fn bench_embedding(c: &mut Criterion) {
    let mut group = c.benchmark_group("embedding");

    // Generate test embeddings
    let dims = 1536; // OpenAI embedding dimension
    let create_embedding = |seed: u32| -> Vec<f32> {
        (0..dims).map(|i| ((seed * 17 + i as u32) % 1000) as f32 / 1000.0).collect()
    };

    // Single cosine similarity
    let a = create_embedding(1);
    let b = create_embedding(2);
    group.bench_function("cosine_similarity_1536d", |bench| {
        bench.iter(|| embedding::cosine_similarity(black_box(&a), black_box(&b)))
    });

    // Dot product
    group.bench_function("dot_product_1536d", |bench| {
        bench.iter(|| embedding::dot_product(black_box(&a), black_box(&b)))
    });

    // Matrix similarity for different sizes
    for size in [10, 100, 500].iter() {
        let queries: Vec<Vec<f32>> = (0..*size).map(|i| create_embedding(i as u32)).collect();
        let docs: Vec<Vec<f32>> = (0..*size).map(|i| create_embedding(i as u32 + 1000)).collect();

        group.bench_with_input(
            BenchmarkId::new("cosine_similarity_matrix", format!("{}x{}", size, size)),
            &(queries.clone(), docs.clone()),
            |bench, (q, d)| {
                bench.iter(|| embedding::cosine_similarity_matrix(black_box(q), black_box(d)))
            },
        );
    }

    // L2 normalization
    for size in [10, 100, 1000].iter() {
        let mut embeddings: Vec<Vec<f32>> = (0..*size).map(|i| create_embedding(i as u32)).collect();

        group.bench_with_input(
            BenchmarkId::new("normalize_l2", size),
            &embeddings.clone(),
            |bench, e| {
                let mut e = e.clone();
                bench.iter(|| embedding::normalize_l2(black_box(&mut e)))
            },
        );
    }

    group.finish();
}

fn bench_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff");

    // Create test file content
    let create_file = |lines: usize| -> String {
        (0..lines)
            .map(|i| format!("line {} with some content here", i))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let matcher = diff::FuzzyMatcher::new(0.6);

    // String similarity
    let a = "hello world this is a test";
    let b = "hello world this is a tset";
    group.bench_function("string_similarity", |bench| {
        bench.iter(|| matcher.similarity(black_box(a), black_box(b)))
    });

    // Fuzzy line matching
    let file_100 = create_file(100);
    let file_1000 = create_file(1000);

    let patch = r#"@@ -10,3 +10,3 @@
 line 9 with some content here
-line 10 with some content here
+line 10 with MODIFIED content here
 line 11 with some content here
"#;

    group.bench_function("apply_patch_100_lines", |bench| {
        bench.iter(|| matcher.apply_patch(black_box(patch), black_box(&file_100)))
    });

    group.bench_function("apply_patch_1000_lines", |bench| {
        bench.iter(|| matcher.apply_patch(black_box(patch), black_box(&file_1000)))
    });

    group.finish();
}

fn bench_log(c: &mut Criterion) {
    let mut group = c.benchmark_group("log");

    // Create test log content
    let create_log = |lines: usize| -> String {
        (0..lines)
            .map(|i| {
                let level = match i % 4 {
                    0 => "INFO",
                    1 => "DEBUG",
                    2 => "WARN",
                    _ => "ERROR",
                };
                format!("[{}] 2024-01-{:02} {:02}:{:02}:{:02} - Message number {}",
                    level, i % 28 + 1, i % 24, i % 60, i % 60, i)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let log_1k = create_log(1000);
    let log_10k = create_log(10000);

    // Initial parse
    group.bench_function("new_1k_lines", |bench| {
        bench.iter(|| log::LogReader::new(black_box(&log_1k)))
    });

    group.bench_function("new_10k_lines", |bench| {
        bench.iter(|| log::LogReader::new(black_box(&log_10k)))
    });

    // Append
    let mut reader = log::LogReader::new(&log_1k);
    let append_content = create_log(100);
    group.bench_function("append_100_lines", |bench| {
        bench.iter(|| {
            let mut r = reader.clone();
            r.append(black_box(&append_content))
        })
    });

    // Filter
    let reader = log::LogReader::new(&log_10k);
    group.bench_function("filter_10k_lines", |bench| {
        bench.iter(|| reader.filter(black_box("ERROR")))
    });

    // Tail
    group.bench_function("tail_100_from_10k", |bench| {
        bench.iter(|| reader.tail(black_box(100)))
    });

    group.finish();
}

fn bench_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("json");

    // Create NDJSON test data
    let create_ndjson = |lines: usize| -> String {
        (0..lines)
            .map(|i| format!(r#"{{"id":{},"name":"item{}","value":{}}}"#, i, i, i * 10))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let ndjson_100 = create_ndjson(100);
    let ndjson_1k = create_ndjson(1000);

    group.bench_function("parse_ndjson_100", |bench| {
        bench.iter(|| json::parse_ndjson(black_box(ndjson_100.as_bytes())))
    });

    group.bench_function("parse_ndjson_1k", |bench| {
        bench.iter(|| json::parse_ndjson(black_box(ndjson_1k.as_bytes())))
    });

    // JSON-RPC batch
    let create_jsonrpc = |count: usize| -> String {
        (0..count)
            .map(|i| {
                if i % 2 == 0 {
                    format!(r#"{{"jsonrpc":"2.0","id":{},"method":"test","params":{{"x":{}}}}}"#, i, i)
                } else {
                    format!(r#"{{"jsonrpc":"2.0","id":{},"result":{{"value":{}}}}}"#, i, i)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let jsonrpc_100 = create_jsonrpc(100);
    group.bench_function("parse_jsonrpc_100", |bench| {
        bench.iter(|| json::parse_jsonrpc_batch(black_box(jsonrpc_100.as_bytes())))
    });

    group.finish();
}

#[cfg(feature = "chunker")]
fn bench_chunker(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunker");

    // Generate test content
    let python_code = r#"
import os
from pathlib import Path

class FileProcessor:
    """Process files in a directory."""

    def __init__(self, root: str):
        self.root = Path(root)

    def process(self, pattern: str) -> list[str]:
        """Find and process matching files."""
        results = []
        for path in self.root.rglob(pattern):
            content = path.read_text()
            results.append(self.transform(content))
        return results

    def transform(self, content: str) -> str:
        """Transform file content."""
        return content.upper()

def main():
    processor = FileProcessor("/tmp")
    results = processor.process("*.txt")
    for r in results:
        print(r)

if __name__ == "__main__":
    main()
"#;

    let markdown_content = {
        let mut s = String::from("# Title\n\nIntroduction paragraph.\n\n");
        for i in 0..20 {
            s.push_str(&format!("## Section {}\n\nContent for section {}. ", i, i));
            s.push_str("This is some detailed text that fills out the section. ");
            s.push_str("It has multiple sentences to make it realistic.\n\n");
        }
        s
    };

    let text_content = {
        let mut s = String::new();
        for i in 0..50 {
            s.push_str(&format!(
                "This is paragraph {}. It contains multiple sentences. Each sentence adds more content. ",
                i
            ));
            s.push_str("The paragraph should be reasonably sized for benchmarking.\n\n");
        }
        s
    };

    let config = chunker::ChunkConfig::default();

    // Code chunking (Python)
    group.bench_function("chunk_code_python_30lines", |bench| {
        bench.iter(|| {
            chunker::chunk_code(
                black_box(python_code),
                &chunker::Language::Python,
                black_box(&config),
            )
        })
    });

    // Markdown chunking
    group.bench_function("chunk_markdown_20sections", |bench| {
        bench.iter(|| chunker::chunk_markdown(black_box(&markdown_content), black_box(&config)))
    });

    // Text chunking
    group.bench_function("chunk_text_50paragraphs", |bench| {
        bench.iter(|| chunker::chunk_text(black_box(&text_content), black_box(&config)))
    });

    // Auto-detect + chunk
    group.bench_function("chunk_auto_python", |bench| {
        let ct = chunker::ContentType::Code(chunker::Language::Python);
        bench.iter(|| chunker::chunk(black_box(python_code), ct.clone(), black_box(&config)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_embedding,
    bench_diff,
    bench_log,
    bench_json,
);

#[cfg(feature = "chunker")]
criterion_group!(
    chunker_benches,
    bench_chunker,
);

#[cfg(feature = "chunker")]
criterion_main!(benches, chunker_benches);

#[cfg(not(feature = "chunker"))]
criterion_main!(benches);
