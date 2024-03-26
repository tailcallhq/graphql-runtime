use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

fn benchmark_batched_body(c: &mut Criterion) {
    c.bench_function("test_batched_body", |b| {
        b.iter(|| {
            let input = json!({
                "data": [
                    {"user": {"id": "1"}},
                    {"user": {"id": "2"}},
                    {"user": {"id": "3"}},
                    {"user": [
                        {"id": "4"},
                        {"id": "5"}
                        ]
                    },
                ]
            });

            black_box(
                serde_json::to_value(tailcall::json::gather_path_matches(
                    &input,
                    &["data".into(), "user".into(), "id".into()],
                    vec![],
                ))
                .unwrap(),
            );
        })
    });
}

fn benchmark_group_by(c: &mut Criterion) {
    c.bench_function("group_by", |b| {
        b.iter(|| {
            let input = json!({
                "data": [
                    {"type": "A", "value": {"id": "1"}},
                    {"type": "B", "value": {"id": "2"}},
                    {"type": "A", "value": {"id": "3"}},
                    {"type": "A", "value": {"id": "4"}},
                    {"type": "B", "value": {"id": "5"}}
                ]
            });

            let binding = ["data".into(), "type".into()];
            let gathered = tailcall::json::gather_path_matches(&input, &binding, vec![]);

            black_box(serde_json::to_value(tailcall::json::group_by_key(gathered)).unwrap());
        })
    });
}

criterion_group!(benches, benchmark_batched_body, benchmark_group_by);
criterion_main!(benches);
