use std::time::{Duration, Instant};

use num_format::ToFormattedString;

use crate::{
    config::{self, IdNormalized},
    decider::decider_result::DeciderResultStats,
};

static REPORT_PROGRESS_STANDARD: ReportProgressStandard = ReportProgressStandard;

/// Simple methods to track time and report something after a while
pub struct Reporter<'a> {
    last_progress_time: Instant,
    report_progress_after: Duration,
    last_detail_time: Instant,
    report_detail_after: Duration,
    report_progress: &'a (dyn ReportProgress + 'a),
    progress_info: ProgressInfo,
}

// impl<R: ReportProgress> Reporter<R> {
impl<'a> Reporter<'a> {
    pub fn new_default(total: IdNormalized) -> Self {
        Self {
            last_progress_time: std::time::Instant::now(),
            report_progress_after: Duration::new(2, 0),
            last_detail_time: std::time::Instant::now(),
            report_detail_after: Duration::new(30, 0),
            report_progress: &REPORT_PROGRESS_STANDARD,
            progress_info: ProgressInfo::new(total),
        }
    }

    // TODO extent Builder with these fields
    //     pub fn new(
    //         report_progress_every_ms: u32,
    //         report_detail_every_s: u64,
    //         report_progress: &'a impl ReportProgress,
    //         total: IdBig,
    //     ) -> Self {
    //         Self {
    //             report_progress_after: Duration::new(0, report_progress_every_ms * 1_000_000),
    //             report_detail_after: Duration::new(report_detail_every_s, 0),
    //             report_progress,
    //             progress_info: ProgressInfo::new(total),
    //             ..Default::default()
    //         }
    //     }
    //
    //     pub fn default_with_custom_reporter(report_progress: &'a impl ReportProgress) -> Self {
    //         Self {
    //             report_progress,
    //             last_progress_time: std::time::Instant::now(),
    //             report_progress_after: Duration::new(2, 0),
    //             last_detail_time: std::time::Instant::now(),
    //             report_detail_after: Duration::new(30, 0),
    //             report_progress: &REPORT_PROGRESS_STANDARD,
    //             progress_info: Default::default(),
    //             total: 0,
    //         }
    //     }

    /// Builder to initialize required values.
    pub fn builder(total: IdNormalized) -> ReporterBuilder {
        ReporterBuilder::new(total)
    }

    /// Reports progress; this only supports x of y (for percentage).
    /// This should be called when self.is_due_progress returns true. \
    /// Calling this too often may be inefficient as the parameters would be passed needlessly most of the time.
    pub fn report(&mut self, processed: IdNormalized) -> String {
        // store progress with time stamp in progress_info
        self.progress_info.add_progress(processed);
        let s = self
            .report_progress
            .report_progress(processed, &self.progress_info);
        self.reset_last_report_progress_time();

        s
    }

    /// Reports progress with DeciderStats details.
    /// This should be called when self.is_due_progress returns true. \
    /// Calling this every time would be inefficient as the parameters would be passed needlessly most of the time.
    pub fn report_stats(&mut self, processed: IdNormalized, result: &DeciderResultStats) -> String {
        // store progress with time stamp in progress_info
        self.progress_info.add_progress(processed);
        let mut s = String::new();
        if self.is_due_detail() {
            s.push_str(self.report_progress.report_detail(result).as_str());
            self.reset_last_report_detail_time();
        }
        s.push_str(
            self.report_progress
                .report_progress(processed, &self.progress_info)
                .as_str(),
        );
        self.reset_last_report_progress_time();

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

    pub fn progress_calc(&self) -> &ProgressInfo {
        &self.progress_info
    }

    pub fn start_calc(&self) -> Instant {
        self.progress_info.start_time
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

// /// Do not use this, always use new
// impl<'a> Default for Reporter<'a> {
//     fn default() -> Self {
//         Self {
//             last_progress_time: std::time::Instant::now(),
//             report_progress_after: Duration::new(2, 0),
//             last_detail_time: std::time::Instant::now(),
//             report_detail_after: Duration::new(30, 0),
//             report_progress: &REPORT_PROGRESS_STANDARD,
//             progress_info: Default::default(),
//         }
//     }
// }

pub trait ReportProgress {
    fn report_progress(&self, processed: IdNormalized, progress_info: &ProgressInfo) -> String;
    fn report_progress_stats(
        &self,
        decider_result: &DeciderResultStats,
        progress_info: &ProgressInfo,
    ) -> String;
    fn report_detail(&self, result: &DeciderResultStats) -> String;
}

#[derive(Default)]
pub struct ReportProgressStandard;

impl ReportProgressStandard {
    pub fn remaining_estimate_for_minutes(
        &self,
        minutes: usize,
        progress_info: &ProgressInfo,
    ) -> Option<Duration> {
        let average = progress_info.progress_average_per_sec(minutes as u64 * 60);
        match average {
            Some(a) => {
                let processed = progress_info.progress_data.last().unwrap().processed;
                Some(Duration::from_secs_f64(
                    (progress_info.total - processed) as f64 / a,
                ))
            }
            None => None,
        }
    }
}

impl ReportProgress for ReportProgressStandard {
    fn report_detail(&self, result: &DeciderResultStats) -> String {
        format!("\nCurrent result\n{result}")
    }

    fn report_progress(&self, processed: IdNormalized, progress_info: &ProgressInfo) -> String {
        let locale = config::user_locale();
        let percent = (processed as f64 / progress_info.total as f64 * 1000.0).round() / 10.0;
        // estimate time to run
        let dur_total = progress_info.start_time.elapsed();
        let p_per_sec = processed as f64 / dur_total.as_secs_f64();
        let remaining_est_total =
            Duration::from_secs_f64((progress_info.total - processed) as f64 / p_per_sec);
        let remaining_est_1m = self.remaining_estimate_for_minutes(1, progress_info);
        let remaining_est_5m = self.remaining_estimate_for_minutes(5, progress_info);
        if remaining_est_1m.is_none() {
            format!(
                "Working: {} / {} ({percent:.1}%), remaining {}, runtime {}", // , end at {:?}",
                processed.to_formatted_string(&locale),
                progress_info.total.to_formatted_string(&locale),
                format_duration_hhmmss_ms(remaining_est_total, false),
                format_duration_hhmmss_ms(dur_total, false)
            )
        } else if remaining_est_5m.is_none() {
            format!(
                "Working: {} / {} ({percent:.1}%), remaining: total {}, avg 1 min {}, runtime {}", // , end at {:?}",
                processed.to_formatted_string(&locale),
                progress_info.total.to_formatted_string(&locale),
                format_duration_hhmmss_ms(remaining_est_total, false),
                format_duration_hhmmss_ms(remaining_est_1m.unwrap(), false),
                format_duration_hhmmss_ms(dur_total, false)
            )
        } else {
            format!(
                "Working: {} / {} ({percent:.1}%), remaining: total {}, avg 1 min {}, avg 5 min {}, runtime {}", // , end at {:?}",
                processed.to_formatted_string(&locale),
                progress_info.total.to_formatted_string(&locale),
                format_duration_hhmmss_ms(remaining_est_total, false),
                format_duration_hhmmss_ms(remaining_est_1m.unwrap(), false),
                format_duration_hhmmss_ms(remaining_est_5m.unwrap(), false),
                format_duration_hhmmss_ms(dur_total, false)
            )
        }
    }

    fn report_progress_stats(
        &self,
        decider_result: &DeciderResultStats,
        progress_info: &ProgressInfo,
    ) -> String {
        let locale = config::user_locale();
        let percent = (decider_result.num_processed_total() as f64
            / decider_result.num_total_turing_machines() as f64
            * 1000.0)
            .round()
            / 10.0;
        // estimate time to run
        let dur_total = progress_info.start_time.elapsed();
        let p_per_sec = decider_result.num_processed_total() as f64 / dur_total.as_secs_f64();
        let remaining = Duration::from_secs_f64(
            (decider_result.num_total_turing_machines() - decider_result.num_processed_total())
                as f64
                / p_per_sec,
        );
        format!(
            "Working: {} / {} ({percent:.1}%), remaining {}, runtime {}", // , end at {:?}",
            decider_result
                .num_processed_total()
                .to_formatted_string(&locale),
            decider_result
                .num_total_turing_machines()
                .to_formatted_string(&locale),
            format_duration_hhmmss_ms(remaining, false),
            format_duration_hhmmss_ms(dur_total, false),
            // TODO some data
        )
    }
}

/// Stores progress of different times to adjust remaining time calculation
#[derive(Debug)]
pub struct ProgressTimeStamp {
    time_stamp: Instant,
    processed: IdNormalized,
}

impl ProgressTimeStamp {
    pub fn time_stamp(&self) -> Instant {
        self.time_stamp
    }

    pub fn processed(&self) -> u64 {
        self.processed
    }
}

#[derive(Debug)]
pub struct ProgressInfo {
    start_time: Instant,
    total: IdNormalized,
    progress_data: Vec<ProgressTimeStamp>,
    /// The maximum duration which is kept, e.g. 600 means all data older than 10 Minutes is deleted
    max_duration_s: u64,
}

impl ProgressInfo {
    pub fn new(total: IdNormalized) -> Self {
        Self {
            start_time: Instant::now(),
            total,
            progress_data: Vec::new(),
            max_duration_s: 600,
        }
    }

    pub fn add_progress(&mut self, processed: IdNormalized) {
        self.progress_data.push(ProgressTimeStamp {
            time_stamp: Instant::now(),
            processed,
        });
        #[allow(clippy::manual_is_multiple_of)]
        if self.progress_data.len() % 50 == 0 {
            self.clean_progress();
        }
    }

    // keep only 10 minutes
    fn clean_progress(&mut self) {
        // find first required
        let reference = Instant::now() - Duration::from_secs(self.max_duration_s);
        for (i, p) in self.progress_data.iter().enumerate() {
            if p.time_stamp >= reference {
                self.progress_data.drain(0..i);
                break;
            }
        }
    }

    pub fn progress_average_per_sec(&self, last_secs: u64) -> Option<f64> {
        // search simple
        // TODO half search, which is faster
        let start_ref = Instant::now()
            .checked_sub(Duration::from_secs(last_secs))
            .unwrap();
        for (i, p) in self.progress_data.iter().enumerate() {
            if p.time_stamp >= start_ref {
                if i == 0 {
                    break;
                }
                let p = &self.progress_data[i - 1];
                let last = self.progress_data.last().unwrap();
                let dur = last.time_stamp - p.time_stamp;
                return Some((last.processed - p.processed) as f64 / dur.as_secs_f64());
            }
        }
        None
    }

    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    pub fn progress_data(&self) -> &[ProgressTimeStamp] {
        &self.progress_data
    }

    pub fn total(&self) -> u64 {
        self.total
    }
}

impl Default for ProgressInfo {
    fn default() -> Self {
        Self::new(0)
    }
}

pub struct ReporterBuilder {
    total: IdNormalized,
}

impl ReporterBuilder {
    pub fn new(total: IdNormalized) -> Self {
        Self { total }
    }

    pub fn build(self) -> Reporter<'static> {
        Reporter {
            last_progress_time: std::time::Instant::now(),
            report_progress_after: Duration::new(2, 0),
            last_detail_time: std::time::Instant::now(),
            report_detail_after: Duration::new(30, 0),
            report_progress: &REPORT_PROGRESS_STANDARD,
            progress_info: ProgressInfo::new(self.total),
        }
    }
}

/// Formats a `std::time::Duration` into a string in `HH:mm:ss.ms` format.
///
/// # Arguments
/// * `duration` - The `Duration` to format.
///
/// # Returns
/// A `String` representing the formatted duration.
///
/// # Examples
/// ```
/// use std::time::Duration;
/// use bb_challenge::reporter::format_duration_hhmmss_ms;
///
/// assert_eq!(format_duration_hhmmss_ms(Duration::from_secs(3661), true), "01:01:01.000");
/// assert_eq!(format_duration_hhmmss_ms(Duration::from_millis(123456), true), "00:02:03.456");
/// assert_eq!(format_duration_hhmmss_ms(Duration::from_millis(123456), false), "00:02:03");
/// assert_eq!(format_duration_hhmmss_ms(Duration::from_millis(123556), false), "00:02:04");
/// ```
pub fn format_duration_hhmmss_ms(duration: Duration, display_millis: bool) -> String {
    let total_milliseconds = duration.as_millis();
    let hours = total_milliseconds / (1000 * 60 * 60);
    let minutes = (total_milliseconds % (1000 * 60 * 60)) / (1000 * 60);
    let mut seconds = ((total_milliseconds % (1000 * 60 * 60)) % (1000 * 60)) / 1000;
    let milliseconds = total_milliseconds % 1000;
    if milliseconds >= 500 {
        seconds += 1;
    }

    if display_millis {
        format!("{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}")
    } else {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }
}

pub fn format_duration_reasonable_size(duration_sec: f64) -> String {
    let duration;
    let duration_type;
    if duration_sec > 7200.0 {
        duration = duration_sec / 3600.0;
        duration_type = "hours";
    } else if duration_sec > 120.0 {
        duration = duration_sec / 60.0;
        duration_type = "min";
    } else if duration_sec > 2.0 {
        duration = duration_sec;
        duration_type = "sec"
    } else {
        duration = duration_sec / 1000.0;
        duration_type = "ms"
    }
    format!("{duration:.1} {duration_type}")
}
