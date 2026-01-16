//! Benchmarks for measuring project generation performance.
//!
//! Run with: `cargo bench`
//!
//! These benchmarks measure the time to generate projects from various templates
//! with different levels of complexity.

use baker::cli::{run, Args, SkipConfirm};
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create Args for benchmarking
fn create_bench_args(template: &str, output_dir: PathBuf, answers: Option<&str>) -> Args {
    Args {
        template: template.to_string(),
        output_dir,
        force: true,
        verbose: 0,
        answers: answers.map(|a| a.to_string()),
        answers_file: None,
        skip_confirms: vec![SkipConfirm::All],
        non_interactive: true,
        dry_run: false,
    }
}

/// Benchmark: Simple demo template generation
fn bench_demo_template(c: &mut Criterion) {
    let mut group = c.benchmark_group("demo_template");

    let answers = r#"{"project_name": "benchmark_project", "project_author": "Benchmark Author", "project_slug": "benchmark_project", "use_tests": true}"#;

    group.bench_function("with_tests", |b| {
        b.iter(|| {
            let tmp_dir = TempDir::new().unwrap();
            let args = create_bench_args(
                "examples/demo",
                tmp_dir.path().to_path_buf(),
                Some(answers),
            );
            run(black_box(args)).unwrap();
        });
    });

    let answers_no_tests = r#"{"project_name": "benchmark_project", "project_author": "Benchmark Author", "project_slug": "benchmark_project", "use_tests": false}"#;

    group.bench_function("without_tests", |b| {
        b.iter(|| {
            let tmp_dir = TempDir::new().unwrap();
            let args = create_bench_args(
                "examples/demo",
                tmp_dir.path().to_path_buf(),
                Some(answers_no_tests),
            );
            run(black_box(args)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark: Filters template (tests filter processing)
fn bench_filters_template(c: &mut Criterion) {
    c.bench_function("filters_template", |b| {
        let answers = r#"{"project_name": "My Awesome Project"}"#;

        b.iter(|| {
            let tmp_dir = TempDir::new().unwrap();
            let args = create_bench_args(
                "examples/filters",
                tmp_dir.path().to_path_buf(),
                Some(answers),
            );
            run(black_box(args)).unwrap();
        });
    });
}

/// Benchmark: Loop template with varying numbers of items
fn bench_loop_template(c: &mut Criterion) {
    let mut group = c.benchmark_group("loop_template");

    for num_items in [5, 10, 25, 50].iter() {
        let items: Vec<serde_json::Value> = (0..*num_items)
            .map(|i| serde_json::json!({"name": format!("item_{}", i)}))
            .collect();
        let answers = serde_json::json!({"items": items, "nested": false}).to_string();

        group.throughput(Throughput::Elements(*num_items as u64));
        group.bench_with_input(
            BenchmarkId::new("items", num_items),
            &answers,
            |b, answers| {
                b.iter(|| {
                    let tmp_dir = TempDir::new().unwrap();
                    let args = create_bench_args(
                        "examples/loop",
                        tmp_dir.path().to_path_buf(),
                        Some(answers),
                    );
                    run(black_box(args)).unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Heavy loop template with many items and nested content
fn bench_heavy_loop_template(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_loop_template");
    group.sample_size(20); // Reduce sample size for heavy benchmarks

    for num_items in [10, 25, 50, 100].iter() {
        let items: Vec<serde_json::Value> = (0..*num_items)
            .map(|i| {
                serde_json::json!({
                    "name": format!("module_{}", i),
                    "description": format!("This is a detailed description for module number {}. It contains multiple sentences to simulate real-world content.", i),
                    "version": format!("{}.{}.{}", i / 10, i % 10, i % 5),
                    "enabled": i % 2 == 0,
                    "tags": vec![format!("tag_{}", i), format!("category_{}", i % 5)]
                })
            })
            .collect();
        let answers = serde_json::json!({"items": items}).to_string();

        group.throughput(Throughput::Elements(*num_items as u64));
        group.bench_with_input(
            BenchmarkId::new("modules", num_items),
            &answers,
            |b, answers| {
                b.iter(|| {
                    let tmp_dir = TempDir::new().unwrap();
                    let args = create_bench_args(
                        "tests/templates/heavy_loop",
                        tmp_dir.path().to_path_buf(),
                        Some(answers),
                    );
                    run(black_box(args)).unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Import template (tests template import functionality)
fn bench_import_template(c: &mut Criterion) {
    c.bench_function("import_template", |b| {
        let answers = r#"{"project_name": "imported_project"}"#;

        b.iter(|| {
            let tmp_dir = TempDir::new().unwrap();
            let args = create_bench_args(
                "examples/import",
                tmp_dir.path().to_path_buf(),
                Some(answers),
            );
            run(black_box(args)).unwrap();
        });
    });
}

/// Benchmark: Hooks template (tests pre/post hook execution)
fn bench_hooks_template(c: &mut Criterion) {
    c.bench_function("hooks_template", |b| {
        let answers = r#"{"license": "MIT"}"#;

        b.iter(|| {
            let tmp_dir = TempDir::new().unwrap();
            let args = create_bench_args(
                "examples/hooks",
                tmp_dir.path().to_path_buf(),
                Some(answers),
            );
            run(black_box(args)).unwrap();
        });
    });
}

/// Benchmark: Comparison of all template types
fn bench_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("template_comparison");

    group.bench_function("demo", |b| {
        let answers = r#"{"project_name": "test", "project_author": "Test", "project_slug": "test", "use_tests": true}"#;
        b.iter(|| {
            let tmp_dir = TempDir::new().unwrap();
            let args = create_bench_args("examples/demo", tmp_dir.path().to_path_buf(), Some(answers));
            run(black_box(args)).unwrap();
        });
    });

    group.bench_function("filters", |b| {
        let answers = r#"{"project_name": "test"}"#;
        b.iter(|| {
            let tmp_dir = TempDir::new().unwrap();
            let args = create_bench_args(
                "examples/filters",
                tmp_dir.path().to_path_buf(),
                Some(answers),
            );
            run(black_box(args)).unwrap();
        });
    });

    group.bench_function("loop_small", |b| {
        let answers = r#"{"items": [{"name": "a"}, {"name": "b"}, {"name": "c"}], "nested": false}"#;
        b.iter(|| {
            let tmp_dir = TempDir::new().unwrap();
            let args = create_bench_args("examples/loop", tmp_dir.path().to_path_buf(), Some(answers));
            run(black_box(args)).unwrap();
        });
    });

    group.bench_function("import", |b| {
        let answers = r#"{"project_name": "test"}"#;
        b.iter(|| {
            let tmp_dir = TempDir::new().unwrap();
            let args = create_bench_args(
                "examples/import",
                tmp_dir.path().to_path_buf(),
                Some(answers),
            );
            run(black_box(args)).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_demo_template,
    bench_filters_template,
    bench_loop_template,
    bench_heavy_loop_template,
    bench_import_template,
    bench_hooks_template,
    bench_comparison,
);
criterion_main!(benches);
