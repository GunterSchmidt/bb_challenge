pub fn duration_as_ms_rounded(duration: std::time::Duration) -> f64 {
    (duration.as_nanos() as f64 / 1000.0).round() / 1000.0
}

pub fn user_locale() -> num_format::Locale {
    // TODO get user locale
    num_format::Locale::en
}

/// Returns the number of CPUs to use.  
/// Percent needs to be between 10 and 100%, anything  
/// else will return the full number of CPUs available.
pub fn num_cpus_percentage(percent: usize) -> usize {
    if percent >= 100 {
        return num_cpus::get();
    }

    let cpus = num_cpus::get();

    let n = cpus * percent / 100;
    if n == 0 {
        1
    } else {
        n
    }
}
