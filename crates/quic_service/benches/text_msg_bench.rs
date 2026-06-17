use common::utils::message_types::MSG_TYPE_TEXT;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use quic_service::X25;
use quic_service::models::text_msg::HeadMsg;
use quic_service::msg_service::text_msg_service::{generate_text_msg, get_text_msg};
use std::sync::Arc;
use tokio::sync::Mutex;

fn head_size() -> usize {
    let head = HeadMsg { version: 1, crc: 0, body_len: 0, message_type: MSG_TYPE_TEXT };
    bincode::serialize(&head).unwrap().len()
}

fn make_msg(raw: &[u8]) -> Vec<u8> {
    generate_text_msg(MSG_TYPE_TEXT, raw.to_vec(), "user_b".into(), "user_a".into()).unwrap()
}

fn new_buffer_msg() -> Arc<Mutex<Vec<u8>>> {
    Arc::new(Mutex::new(Vec::new()))
}

fn bench_sticky(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let head_len = head_size();

    let mut group = c.benchmark_group("sticky_packets");
    for n in [1u64, 2, 5, 10, 50, 100] {
        let payload = b"bench_payload_32bytes_ref_data";
        let single = make_msg(payload);
        let single_len = single.len() as u64;

        let mut combined = Vec::with_capacity(single.len() * n as usize);
        for _ in 0..n {
            combined.extend_from_slice(&single);
        }
        let total_len = combined.len();

        group.throughput(Throughput::Bytes(single_len * n));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let mut buf = combined.clone();
                let buf_msg = new_buffer_msg();
                let result = rt.block_on(get_text_msg(&mut buf, total_len, buf_msg, head_len));
                black_box(result).unwrap();
            })
        });
    }
    group.finish();
}

fn bench_body_size(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let head_len = head_size();

    let mut group = c.benchmark_group("body_size");
    for size in [64u64, 256, 1024, 4096, 16384, 65536] {
        let raw = vec![b'X'; size as usize];
        let msg = make_msg(&raw);
        let msg_len = msg.len();

        group.throughput(Throughput::Bytes(msg_len as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let mut buf = msg.clone();
                let buf_msg = new_buffer_msg();
                let result = rt.block_on(get_text_msg(&mut buf, msg_len, buf_msg, head_len));
                black_box(result).unwrap();
            })
        });
    }
    group.finish();
}

fn bench_carryover(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let head_len = head_size();

    let mut group = c.benchmark_group("carryover_reassembly");
    // 分 2、4、8 片到达
    for fragments in [2u64, 4, 8] {
        let payload = b"carryover_benchmark_payload_data_64b";
        let full_msg = make_msg(payload);
        let chunk_size = full_msg.len() / fragments as usize;

        group.throughput(Throughput::Bytes(full_msg.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("fragments", fragments),
            &fragments,
            |b, &fragments| {
                b.iter(|| {
                    let buf_msg = new_buffer_msg();
                    let fragments = fragments as usize;
                    let mut total_parsed = 0;

                    for i in 0..fragments {
                        let start = i * chunk_size;
                        let end =
                            if i == fragments - 1 { full_msg.len() } else { (i + 1) * chunk_size };
                        let mut chunk = full_msg[start..end].to_vec();
                        let chunk_len = chunk.len();
                        let result = rt
                            .block_on(get_text_msg(
                                &mut chunk,
                                chunk_len,
                                buf_msg.clone(),
                                head_len,
                            ))
                            .unwrap();
                        total_parsed += result.len();
                    }
                    black_box(total_parsed);
                })
            },
        );
    }
    group.finish();
}

fn bench_crc_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("crc_checksum");
    for size in [64u64, 256, 1024, 4096, 16384, 65536] {
        let data = vec![0xABu8; size as usize];
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| black_box(X25.checksum(black_box(&data))))
        });
    }
    group.finish();
}

fn bench_head_deserialize(c: &mut Criterion) {
    let head = HeadMsg { version: 1, crc: 12345, body_len: 1024, message_type: MSG_TYPE_TEXT };
    let head_bytes = bincode::serialize(&head).unwrap();

    c.bench_function("head_deserialize", |b| {
        b.iter(|| {
            let h: HeadMsg = bincode::deserialize(black_box(&head_bytes)).unwrap();
            black_box(h);
        })
    });
}

fn bench_build_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_message");
    for size in [64u64, 256, 1024, 4096, 16384] {
        let raw = vec![b'Y'; size as usize];
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                black_box(generate_text_msg(
                    MSG_TYPE_TEXT,
                    black_box(raw.clone()),
                    "user_b".into(),
                    "user_a".into(),
                ))
                .unwrap();
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_sticky,
    bench_body_size,
    bench_carryover,
    bench_crc_overhead,
    bench_head_deserialize,
    bench_build_message,
);
criterion_main!(benches);
