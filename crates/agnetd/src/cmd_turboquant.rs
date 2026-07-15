use anyhow::Result;

use crate::cli::{GlobalArgs, TurboquantCommands};
use crate::output::OutputWriter;
use crate::turboquant_service::{self, CodebookParams, CompressionParams};

pub fn execute(command: &TurboquantCommands, args: &GlobalArgs) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match command {
        TurboquantCommands::Compress {
            profile,
            workers,
            bandwidth_mbps,
            target_bits,
            bits,
            dimension,
        } => {
            let result = turboquant_service::select_strategy(CompressionParams {
                profile: profile.clone(),
                workers: *workers,
                bandwidth_mbps: *bandwidth_mbps,
                target_bits: *target_bits,
                bits: *bits,
                dimension: *dimension,
            })?;
            writer.write_json(&result);
            Ok(())
        }
        TurboquantCommands::GenerateCodebook {
            bits,
            dimension,
            max_iterations,
            convergence_threshold,
            num_samples,
        } => {
            let result = turboquant_service::generate_codebook(CodebookParams {
                bits: *bits,
                dimension: *dimension,
                max_iterations: *max_iterations,
                convergence_threshold: *convergence_threshold,
                num_samples: *num_samples,
            })?;
            writer.write_json(&result);
            Ok(())
        }
    }
}
