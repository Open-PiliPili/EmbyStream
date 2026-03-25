use std::time::Instant;

/// Helper for timing operations and logging slow executions.
pub struct OperationTimer {
    start: Instant,
    threshold_ms: u128,
}

impl OperationTimer {
    pub fn new(_operation: &'static str, threshold_ms: u128) -> Self {
        Self {
            start: Instant::now(),
            threshold_ms,
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    pub fn is_slow(&self) -> bool {
        self.elapsed_ms() >= self.threshold_ms
    }

    pub fn finish(self) -> u128 {
        self.elapsed_ms()
    }
}

/// Macro to time an operation and log if it exceeds threshold.
#[macro_export]
macro_rules! time_operation {
    ($domain:expr, $operation:expr, $threshold_ms:expr, $code:block) => {{
        let _timer_start = std::time::Instant::now();
        let _timer_result = $code;
        let _timer_elapsed = _timer_start.elapsed().as_millis();
        if _timer_elapsed >= $threshold_ms {
            $crate::warn_log!(
                $domain,
                "slow_operation op={} elapsed_ms={} threshold_ms={}",
                $operation,
                _timer_elapsed,
                $threshold_ms
            );
        } else {
            $crate::debug_log!(
                $domain,
                "operation_complete op={} elapsed_ms={}",
                $operation,
                _timer_elapsed
            );
        }
        _timer_result
    }};
    ($domain:expr, $operation:expr, $threshold_ms:expr, $request_id:expr, $code:block) => {{
        let _timer_start = std::time::Instant::now();
        let _timer_result = $code;
        let _timer_elapsed = _timer_start.elapsed().as_millis();
        if _timer_elapsed >= $threshold_ms {
            $crate::warn_log!(
                $domain,
                "slow_operation request_id={} op={} elapsed_ms={} threshold_ms={}",
                $request_id,
                $operation,
                _timer_elapsed,
                $threshold_ms
            );
        } else {
            $crate::debug_log!(
                $domain,
                "operation_complete request_id={} op={} elapsed_ms={}",
                $request_id,
                $operation,
                _timer_elapsed
            );
        }
        _timer_result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_tracks_elapsed_time() {
        let timer = OperationTimer::new("test_op", 100);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10);
        assert!(elapsed < 50);
    }

    #[test]
    fn timer_detects_slow_operations() {
        let timer = OperationTimer::new("test_op", 5);
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(timer.is_slow());
    }

    #[test]
    fn timer_detects_fast_operations() {
        let timer = OperationTimer::new("test_op", 1000);
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(!timer.is_slow());
    }
}
