use std::time::Duration;

pub trait DurationOps {
    fn div(duration: Duration, divisor: Self) -> Duration;
}

impl DurationOps for f32 {
    fn div(duration: Duration, divisor: Self) -> Duration {
        duration.div_f32(divisor)
    }
}

impl DurationOps for f64 {
    fn div(duration: Duration, divisor: Self) -> Duration {
        duration.div_f64(divisor)
    }
}
