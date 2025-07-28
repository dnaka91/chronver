use std::hint::black_box;

fn main() {
    divan::main();
}

#[divan::bench(args = [
    "1.2.3",
    "1.2.3-pre",
    "1.2.3+abc",
    "1.2.3-pre+abc",
])]
fn semver(value: &str) -> semver::Version {
    semver::Version::parse(black_box(value)).unwrap()
}

#[divan::bench(args = [
    "2019.01.06",
    "2019.01.06.1",
    "2019.01.06-test",
    "2019.01.06.1-test",
    "2019.01.06.1-test.1",
])]
fn chronver(value: &str) -> chronver::Version {
    chronver::Version::parse(black_box(value)).unwrap()
}
