extern crate htmd;

use std::{str::FromStr, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};

macro_rules! concat_strings {
    ($($x:expr),*) => {{
        let mut len = 0;
        $(
            len += &$x.len();
        )*
        let mut result = String::with_capacity(len);
        $(
            result.push_str(&$x);
        )*
        result
    }};
}

fn benchmark(c: &mut Criterion) {
    let a = String::from_str("Hello World").unwrap();
    let b = String::from_str("Lorem ipsum").unwrap();
    let z = "Less is more";

    c.bench_function("format!()", |bencher| {
        bencher.iter(|| format!("{}{}{}", a, b, z))
    });

    c.bench_function("concat_strings!()", |bencher| {
        bencher.iter(|| concat_strings!(a, b, z))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(20));
    targets = benchmark
);
criterion_main!(benches);
