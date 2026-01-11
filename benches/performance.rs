use criterion::{black_box, criterion_group, criterion_main, Criterion};

// We can't easily benchmark the GUI parts, but we can benchmark the text buffer
// by creating a minimal version here

use ropey::Rope;

fn benchmark_insert(c: &mut Criterion) {
    c.bench_function("insert_1000_chars", |b| {
        b.iter(|| {
            let mut rope = Rope::new();
            for i in 0..1000 {
                rope.insert_char(i, 'a');
            }
            black_box(rope)
        })
    });
}

fn benchmark_large_insert(c: &mut Criterion) {
    c.bench_function("insert_middle_large_file", |b| {
        let rope = Rope::from_str(&"a".repeat(100_000));
        b.iter(|| {
            let mut r = rope.clone();
            r.insert_char(50_000, 'X');
            black_box(r)
        })
    });
}

fn benchmark_navigation(c: &mut Criterion) {
    c.bench_function("navigate_100k_lines", |b| {
        let text = (0..100_000).map(|i| format!("Line {}\n", i)).collect::<String>();
        let rope = Rope::from_str(&text);
        b.iter(|| {
            let line = rope.char_to_line(500_000);
            let char_idx = rope.line_to_char(line);
            black_box((line, char_idx))
        })
    });
}

criterion_group!(benches, benchmark_insert, benchmark_large_insert, benchmark_navigation);
criterion_main!(benches);
