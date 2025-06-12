use std::io::Write;

use sov_metrics::Metric;

pub fn track_sequence_number(sequence_number: u64) {
    sov_metrics::track_metrics(|tracker| {
        tracker.submit_inline(
            "sov_rollup_current_sequence_number",
            format!("current_sequence_number={}", sequence_number),
        );
    });
}

pub fn track_in_progress_batch_size(num_txs: u64) {
    sov_metrics::track_metrics(|tracker| {
        tracker.submit_inline(
            "sov_rollup_in_progress_batch_size",
            format!("num_txs={}", num_txs),
        );
    });
}

#[derive(Debug)]
pub struct PreferredSequencerUpdateStateMetrics {
    pub duration: std::time::Duration,
    pub lock_duration: std::time::Duration,
    pub batches_count: u64,
    pub transactions_count: u64,
    pub in_progress_batch: bool,
}

impl Metric for PreferredSequencerUpdateStateMetrics {
    fn measurement_name(&self) -> &'static str {
        "sov_rollup_preferred_sequencer_update_state"
    }

    fn serialize_for_telegraf(&self, buffer: &mut Vec<u8>) -> std::io::Result<()> {
        write!(
            buffer,
            "{} duration_ms={},lock_duration_ms={},batches_count={},transactions_count={},in_progress_batch={}",
            self.measurement_name(),
            self.duration.as_millis(),
            self.lock_duration.as_millis(),
            self.batches_count,
            self.transactions_count,
            self.in_progress_batch
        )
    }
}
