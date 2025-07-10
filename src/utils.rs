pub fn duration_as_ms_rounded(duration: std::time::Duration) -> f64 {
    (duration.as_nanos() as f64 / 1000.0).round() / 1000.0
}

/// Returns the number of CPUs to use. \
/// Percent needs to be between 0 and 150%. \  
/// Returns number of cpus to use; at least 1 cpu, at most 1.5 * available CPUs (110% can be better to actually utilize 100%).
pub fn num_cpus_percentage(percent: usize) -> usize {
    if percent >= 150 {
        return num_cpus::get() * 3 / 2;
    }

    let cpus = num_cpus::get();

    let n = cpus * percent / 100;
    if n == 0 {
        1
    } else {
        n
    }
}

// check if a file exists
pub fn file_exists(file_path: &str) -> bool {
    std::path::Path::new(file_path).exists()
}
