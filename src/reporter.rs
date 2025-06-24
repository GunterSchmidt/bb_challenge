use std::time::{Duration, Instant};

use num_format::ToFormattedString;

use crate::decider_result::DeciderResultStats;

static REPORT_PROGRESS_STANDARD: ReportProgressStandard = ReportProgressStandard;

/// Simple methods to track time and report something after a while
pub struct Reporter<'a> {
    pub start_calc: Instant,
    last_progress_time: Instant,
    report_progress_after: Duration,
    last_detail_time: Instant,
    report_detail_after: Duration,
    report_progress: &'a (dyn ReportProgress + 'a),
}

// impl<R: ReportProgress> Reporter<R> {
impl<'a> Reporter<'a> {
    pub fn new(
        report_progress_every_ms: u32,
        report_detail_every_s: u64,
        report_progress: &'a impl ReportProgress,
    ) -> Self {
        Self {
            start_calc: std::time::Instant::now(),
            report_progress_after: Duration::new(0, report_progress_every_ms * 1_000_000),
            report_detail_after: Duration::new(report_detail_every_s, 0),
            report_progress: report_progress,
            last_progress_time: Instant::now(),
            last_detail_time: Instant::now(),
        }
    }

    pub fn default_with_custom_reporter(report_progress: &'a impl ReportProgress) -> Self {
        Self {
            start_calc: std::time::Instant::now(),
            report_progress_after: Duration::new(2, 0),
            report_detail_after: Duration::new(30, 0),
            report_progress: report_progress,
            last_progress_time: Instant::now(),
            last_detail_time: Instant::now(),
        }
    }

    /// This should be called when self.is_due_progress returns true. \
    /// Calling this every time would be inefficient as the parameters would be passed needlessly most of the time.
    pub fn report(&mut self, processed: u64, total: u64, result: &DeciderResultStats) -> String {
        let mut s = self
            .report_progress
            .report_progress(processed, total, self.start_calc);
        self.reset_last_report_progress_time();

        if self.is_due_detail() {
            s.push_str(self.report_progress.report_detail(result).as_str());
            self.reset_last_report_detail_time();
        }

        s
    }

    /// After info was reported, the time needs to be reset for due calculation.
    pub fn reset_last_report_progress_time(&mut self) {
        self.last_progress_time = std::time::Instant::now()
    }

    pub fn reset_last_report_detail_time(&mut self) {
        self.last_detail_time = std::time::Instant::now()
    }

    pub fn is_due_progress(&self) -> bool {
        self.last_progress_time.elapsed() > self.report_progress_after
    }

    pub fn is_due_detail(&self) -> bool {
        self.last_detail_time.elapsed() > self.report_detail_after
    }

    pub fn last_report_progress_time(&self) -> Instant {
        self.last_progress_time
    }

    pub fn report_progress_after(&self) -> Duration {
        self.report_progress_after
    }

    pub fn last_report_detail_time(&self) -> Instant {
        self.last_detail_time
    }

    pub fn report_detail_after(&self) -> Duration {
        self.report_detail_after
    }

    // pub fn check_time(&self) {
    //     if self.last_reporting.elapsed() > self.report_after {
    //         let mio = (result.num_total as f64 / 100_000.0).round() / 10.0;
    //         let p = (result.num_total as f64 / total_to_check as f64 * 1000.0).round() / 10.0;
    //         println!("Working: {} = {} million, {p}%", result.num_total, mio);
    //         self.last_reporting = std::time::Instant::now();
    //         if self.last_result.elapsed() > self.result_after {
    //             println!("Current result {}", self.result);
    //             self.last_result = std::time::Instant::now();
    //         }
    //     }
    // }
}

impl<'a> Default for Reporter<'a> {
    fn default() -> Self {
        Self {
            start_calc: std::time::Instant::now(),
            last_progress_time: std::time::Instant::now(),
            report_progress_after: Duration::new(2, 0),
            last_detail_time: std::time::Instant::now(),
            report_detail_after: Duration::new(30, 0),
            report_progress: &REPORT_PROGRESS_STANDARD,
        }
    }
}

pub trait ReportProgress {
    fn report_progress(&self, processed: u64, total: u64, start: Instant) -> String;
    fn report_detail(&self, result: &DeciderResultStats) -> String;
}

#[derive(Default)]
pub struct ReportProgressStandard;

impl ReportProgress for ReportProgressStandard {
    fn report_progress(&self, processed: u64, total: u64, start: Instant) -> String {
        let locale = crate::utils::user_locale();
        let percent = (processed as f64 / total as f64 * 1000.0).round() / 10.0;
        // estimate time to run
        let dur_total = start.elapsed().as_secs_f64();
        let p_per_sec = processed as f64 / dur_total;
        let remaining = (total - processed) as f64 / p_per_sec;
        format!(
            "Working: {} / {} ({percent:.1}%), left {}, runtime {}", // , end at {:?}",
            processed.to_formatted_string(&locale),
            total.to_formatted_string(&locale),
            fmt_duration(remaining),
            fmt_duration(dur_total),
        )
    }

    fn report_detail(&self, result: &DeciderResultStats) -> String {
        format!("\nCurrent result\n{}", result)
    }
}

fn fmt_duration(duration_sec: f64) -> String {
    let remaining;
    let remaining_type;
    if duration_sec > 7200.0 {
        remaining = duration_sec / 3600.0;
        remaining_type = "hours";
    } else if duration_sec > 120.0 {
        remaining = duration_sec / 60.0;
        remaining_type = "min";
    } else {
        remaining = duration_sec;
        remaining_type = "sec"
    }
    format!("{remaining:.1} {remaining_type}")
}
