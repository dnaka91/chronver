use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

pub fn parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("Parse");

    for i in ["1.2.3", "1.2.3-pre", "1.2.3+abc", "1.2.3-pre+abc"].iter() {
        group.bench_with_input(BenchmarkId::new("SemVer", i), i, |b, i| {
            b.iter(|| semver::Version::parse(black_box(i)))
        });
    }

    for i in [
        "2019.01.06",
        "2019.01.06.1",
        "2019.01.06-test",
        "2019.01.06.1-test",
        "2019.01.06.1-test.1",
    ]
    .iter()
    {
        group.bench_with_input(BenchmarkId::new("ChronVer", i), i, |b, i| {
            b.iter(|| chronver::Version::parse(black_box(i)))
        });
    }

    group.finish();
}

criterion_group!(benches, parse);
criterion_main!(benches);
