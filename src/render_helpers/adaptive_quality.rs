use std::collections::VecDeque;

#[derive(Debug)]
pub struct AdaptiveQualityController {
    frame_times: VecDeque<f64>,
    capacity: usize,
    quality: u8,
    upgrade_streak: usize,
    upgrade_threshold: f64,
    downgrade_threshold: f64,
    upgrade_streak_required: usize,
}

impl Default for AdaptiveQualityController {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptiveQualityController {
    pub fn new() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(60),
            capacity: 60,
            quality: 2,
            upgrade_streak: 0,
            upgrade_threshold: 8.0,
            downgrade_threshold: 12.0,
            upgrade_streak_required: 30,
        }
    }

    pub fn record_frame(&mut self, frame_duration_ms: f64) -> u8 {
        if self.frame_times.len() == self.capacity {
            self.frame_times.pop_front();
        }
        self.frame_times.push_back(frame_duration_ms);

        if self.frame_times.len() < 10 {
            return self.quality;
        }

        let avg = self.average_frame_time();

        if avg > self.downgrade_threshold {
            if self.quality > 0 {
                self.quality -= 1;
            }
            self.upgrade_streak = 0;
            return self.quality;
        }

        if frame_duration_ms < self.upgrade_threshold {
            self.upgrade_streak += 1;
            if self.upgrade_streak >= self.upgrade_streak_required && self.quality < 2 {
                self.quality += 1;
                self.upgrade_streak = 0;
            }
        } else {
            self.upgrade_streak = 0;
        }

        self.quality
    }

    fn average_frame_time(&self) -> f64 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64
    }

    pub fn quality(&self) -> u8 {
        self.quality
    }

    pub fn reset(&mut self) {
        self.quality = 2;
        self.frame_times.clear();
        self.upgrade_streak = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_initial_quality() {
        let controller = AdaptiveQualityController::new();
        assert_eq!(controller.quality(), 2);
    }

    #[test]
    fn test_record_frame_returns_quality() {
        let mut controller = AdaptiveQualityController::new();
        let quality = controller.record_frame(5.0);
        assert!(quality <= 2);
    }

    #[test]
    fn test_downgrade_on_slow_frames() {
        let mut controller = AdaptiveQualityController::new();
        for _ in 0..20 {
            controller.record_frame(15.0);
        }
        assert!(controller.quality() < 2);
    }

    #[test]
    fn test_upgrade_requires_consecutive_good_frames() {
        let mut controller = AdaptiveQualityController::new();
        for _ in 0..20 {
            controller.record_frame(15.0);
        }
        let initial_quality = controller.quality();
        assert!(initial_quality < 2);

        for _ in 0..90 {
            controller.record_frame(5.0);
        }
        assert_eq!(controller.quality(), 2);
    }

    #[test]
    fn test_reset_restores_quality() {
        let mut controller = AdaptiveQualityController::new();
        for _ in 0..20 {
            controller.record_frame(15.0);
        }
        assert!(controller.quality() < 2);

        controller.reset();
        assert_eq!(controller.quality(), 2);
    }

    #[test]
    fn test_reset_clears_statistics() {
        let mut controller = AdaptiveQualityController::new();
        for _ in 0..10 {
            controller.record_frame(5.0);
        }

        controller.reset();

        assert!(controller.frame_times.is_empty());
    }

    #[test]
    fn test_no_downgrade_at_threshold_boundary() {
        let mut controller = AdaptiveQualityController::new();
        for _ in 0..20 {
            controller.record_frame(11.9);
        }
        assert_eq!(controller.quality(), 2);
    }

    #[test]
    fn test_downgrade_at_threshold() {
        let mut controller = AdaptiveQualityController::new();
        for _ in 0..20 {
            controller.record_frame(12.1);
        }
        assert!(controller.quality() < 2);
    }

    #[test]
    fn test_upgrade_threshold_streak_broken() {
        let mut controller = AdaptiveQualityController::new();
        for _ in 0..20 {
            controller.record_frame(15.0);
        }
        assert_eq!(controller.quality(), 0);

        for _ in 0..15 {
            controller.record_frame(5.0);
        }
        assert_eq!(controller.quality(), 0);

        controller.record_frame(10.0);

        for _ in 0..30 {
            controller.record_frame(5.0);
        }
        assert_eq!(controller.quality(), 1);
    }

    #[test]
    fn test_quality_cannot_go_below_zero() {
        let mut controller = AdaptiveQualityController::new();
        for _ in 0..100 {
            controller.record_frame(20.0);
        }
        assert_eq!(controller.quality(), 0);
    }

    #[test]
    fn test_quality_cannot_exceed_two() {
        let mut controller = AdaptiveQualityController::new();
        for _ in 0..100 {
            controller.record_frame(5.0);
        }
        assert_eq!(controller.quality(), 2);
    }
}
