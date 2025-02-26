use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::thread;
use switflake::Switflake;

fn bench_generate_id_single_thread(c: &mut Criterion) {
    let mut swit = Switflake::new(1).expect("Failed to create Switflake");
    c.bench_function("generate_id_single_thread", |b| {
        b.iter(|| swit.generate_id().expect("Failed to generate ID"))
    });
}

fn bench_generate_id_multi_thread(c: &mut Criterion) {
    c.bench_function("generate_id_multi_thread", |b| {
        b.iter(|| {
            let mut handles = Vec::new();
            for _ in 0..8 {
                let mut swit = Switflake::new(1).expect("Failed to create Switflake");
                handles.push(thread::spawn(move || {
                    for _ in 0..64 {
                        let _ = swit.generate_id().expect("Failed to generate ID");
                    }
                }));
            }
            for handle in handles {
                handle.join().expect("Thread join failed");
            }
        })
    });
}

criterion_group!(
    benches,
    bench_generate_id_single_thread,
    bench_generate_id_multi_thread
);
criterion_main!(benches);
