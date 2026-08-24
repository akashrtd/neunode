use std::hint::black_box;
use std::time::{Duration, Instant};

use neunode_turboquant::{Int8Quantizer, MseConfig, MseQuantizer, RotationStrategy};

const DIMENSION: usize = 1 << 20;
const ITERATIONS: usize = 20;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gradients: Vec<f32> = (0..DIMENSION)
        .map(|i| {
            let x = i as f32;
            (x * 0.013).sin() * 0.8 + (x * 0.007).cos() * 0.2
        })
        .collect();

    let baseline = measure(|| Ok::<_, std::convert::Infallible>(black_box(gradients.clone())))?;

    let int8 = Int8Quantizer::new_auto(&gradients)?;
    let int8_result = int8.quantize(&gradients)?;
    let int8_recovered = int8.dequantize(&int8_result);
    let int8_time = measure(|| int8.quantize(black_box(&gradients)))?;

    let tq = MseQuantizer::new(MseConfig {
        dimension: DIMENSION,
        bits: 1,
        rotation_strategy: RotationStrategy::Wht,
        seed: 42,
    })?;
    let tq_result = tq.compress(&gradients)?;
    let tq_recovered = tq.decompress(&tq_result)?;
    let tq_time = measure(|| tq.compress(black_box(&gradients)))?;

    println!("method,payload_bytes,compression_ratio,mse,cosine_similarity,encode_ms");
    print_row("f32", DIMENSION * 4, 1.0, 0.0, 1.0, baseline);
    print_row(
        "int8",
        DIMENSION + size_of::<f32>(),
        4.0,
        mse(&gradients, &int8_recovered),
        cosine_similarity(&gradients, &int8_recovered),
        int8_time,
    );
    print_row(
        "tq_mse_1bit",
        DIMENSION.div_ceil(8) + size_of::<f32>() + size_of::<u64>(),
        32.0,
        mse(&gradients, &tq_recovered),
        cosine_similarity(&gradients, &tq_recovered),
        tq_time,
    );
    eprintln!(
        "note: tq_mse_1bit payload is the wire-size target; CompressedVector currently stores one u32 per index"
    );
    Ok(())
}

fn measure<T, E>(mut operation: impl FnMut() -> Result<T, E>) -> Result<Duration, E> {
    let started = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(operation()?);
    }
    Ok(started.elapsed() / ITERATIONS as u32)
}

fn print_row(
    name: &str,
    bytes: usize,
    ratio: f64,
    error: f64,
    similarity: f64,
    duration: Duration,
) {
    println!(
        "{name},{bytes},{ratio:.1},{error:.8},{similarity:.8},{:.3}",
        duration.as_secs_f64() * 1_000.0
    );
}

fn mse(left: &[f32], right: &[f32]) -> f64 {
    left.iter().zip(right).map(|(a, b)| (*a as f64 - *b as f64).powi(2)).sum::<f64>()
        / left.len() as f64
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    let dot = left.iter().zip(right).map(|(a, b)| *a as f64 * *b as f64).sum::<f64>();
    let left_norm = left.iter().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt();
    let right_norm = right.iter().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt();
    dot / (left_norm * right_norm)
}
