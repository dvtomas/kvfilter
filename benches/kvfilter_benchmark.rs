#[macro_use]
extern crate criterion;
#[macro_use]
extern crate slog;
extern crate slog_kvfilter;

use std::iter::FromIterator;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use criterion::Criterion;
use slog::{Drain, Level, Logger, Never, OwnedKVList, Record};
use slog_kvfilter::{KVFilter, KVFilterList};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct CountingDrain {
    count: Arc<AtomicUsize>,
}

impl Drain for CountingDrain {
    type Ok = ();
    type Err = Never;

    fn log(&self, _: &Record, _: &OwnedKVList) -> Result<(), Never> {
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    #[inline]
    fn is_enabled(&self, _: Level) -> bool {
        true
    }
}

struct Tester {
    log: Logger,
    count: Arc<AtomicUsize>,
}

impl Tester {
    fn assert_count(&self, expected_count: usize) {
        let actual_count = self.count.load(Ordering::Relaxed);
        assert_eq!(expected_count, actual_count)
    }
}

fn new_tester(filters: Option<KVFilterList>, neg_filters: Option<KVFilterList>) -> Tester {
    let count = Arc::new(AtomicUsize::new(0));
    let filter = KVFilter::new(
        CountingDrain {
            count: Arc::clone(&count),
        },
        Level::Info,
    ).only_pass_any_on_all_keys(filters)
        .always_suppress_any(neg_filters);

    Tester {
        log: Logger::root(filter.fuse(), o!("key_foo" => "value_foo")),
        count,
    }
}

// simple AND use_case - useful for comparison with original KVFilter in simple cases
fn simple_and_benchmark(c: &mut Criterion) {
    let tester = new_tester(
        Some(
            vec![
                (
                    "some_key".to_string(),
                    HashSet::from_iter(vec!["some_value".to_string()]),
                ),
                (
                    "another_key".to_string(),
                    HashSet::from_iter(vec!["another_value".to_string()]),
                ),
            ].into_iter()
                .collect(),
        ),
        None,
    );

    let mut first_iteration = true;
    c.bench_function("simple AND", move |b| {
        b.iter(|| {
            info!(tester.log, "ACCEPT";
                "some_key" => "some_value",
                "another_key" => "another_value",
            );

            debug!(tester.log, "REJECT";
                "some_key" => "some_value",
            );

            trace!(tester.log, "REJECT";
                "another_key" => "another_value",
                "bad_key" => "bad_key",
            );

            if first_iteration {
                tester.assert_count(1);
                first_iteration = false;
            }
        })
    });
}

// @przygienda use-case
fn przygienda_tester() -> Tester {
    new_tester(
        Some(
            vec![
                (
                    "some_key".to_string(),
                    HashSet::from_iter(vec![
                        "some_value_1".to_string(),
                        "some_value_2".to_string(),
                        "some_value_3".to_string(),
                        "some_value_4".to_string(),
                        "foo".to_string(),
                    ]),
                ),
                (
                    "another_key".to_string(),
                    HashSet::from_iter(vec![
                        "another_value_1".to_string(),
                        "another_value_2".to_string(),
                        "another_value_3".to_string(),
                        "another_value_4".to_string(),
                        "bar".to_string(),
                    ]),
                ),
                (
                    "key_foo".to_string(),
                    HashSet::from_iter(vec![
                        "foo_value_1".to_string(),
                        "foo_value_2".to_string(),
                        "foo_value_3".to_string(),
                        "foo_value_4".to_string(),
                        "value_foo".to_string(),
                    ]),
                ),
                (
                    "bar_key".to_string(),
                    HashSet::from_iter(vec![
                        "bar_value_1".to_string(),
                        "bar_value_2".to_string(),
                        "bar_value_3".to_string(),
                        "bar_value_4".to_string(),
                        "xyz".to_string(),
                    ]),
                ),
                (
                    "ultimate_key".to_string(),
                    HashSet::from_iter(vec![
                        "ultimate_value_1".to_string(),
                        "ultimate_value_2".to_string(),
                        "ultimate_value_3".to_string(),
                        "ultimate_value_4".to_string(),
                        "xyz".to_string(),
                    ]),
                ),
            ].into_iter().collect(),
        ),
        Some(
            vec![
                (
                    "some_negative_key".to_string(),
                    HashSet::from_iter(vec![
                        "some_value_1".to_string(),
                        "some_value_2".to_string(),
                        "some_value_3".to_string(),
                        "some_value_4".to_string(),
                        "foo".to_string(),
                    ]),
                ),
                (
                    "another_negative_key".to_string(),
                    HashSet::from_iter(vec![
                        "some_value_1".to_string(),
                        "some_value_2".to_string(),
                        "some_value_3".to_string(),
                        "some_value_4".to_string(),
                        "foo".to_string(),
                    ]),
                ),
            ].into_iter().collect(),
        ),
    )
}

fn przygienda_benchmark(c: &mut Criterion) {
    let tester = przygienda_tester();
    let mut first_iteration = true;
    c.bench_function("przygienda", move |b| {
        b.iter(|| {
            info!(tester.log, "ACCEPT";
                "some_key" => "some_value_4",
                "another_key" => "another_value_1",
                "bar_key" => "bar_value_3",
                "ultimate_key" => "ultimate_value_3",
            );

            info!(tester.log, "REJECT - negative filter";
                "some_key" => "some_value_4",
                "another_key" => "another_value_1",
                "bar_key" => "bar_value_3",
                "ultimate_key" => "ultimate_value_3",
                "some_negative_key" => "foo"
            );

            info!(tester.log, "REJECT - not all keys present";
                "some_key" => "some_value_4",
                "another_key" => "another_value_1",
            );

            if first_iteration {
                tester.assert_count(1);
                first_iteration = false;
            }
        })
    });
}

criterion_group!(benches, simple_and_benchmark, przygienda_benchmark);
criterion_main!(benches);
