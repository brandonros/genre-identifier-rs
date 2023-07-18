use governor::Quota;
use governor::RateLimiter;
use governor::clock::Clock;
use governor::clock::QuantaClock;
use governor::clock::QuantaInstant;
use governor::middleware::NoOpMiddleware;
use governor::state::InMemoryState;
use governor::state::NotKeyed;

pub struct RateLimiterWrapper {
    clock: QuantaClock,
    limiter: RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>,
}

impl RateLimiterWrapper {
    pub fn new(rate: u32) -> Self {
        let clock = QuantaClock::default();
        let max_burst = std::num::NonZeroU32::new(rate).unwrap();
        let quota = Quota::per_second(max_burst);
        let limiter = RateLimiter::direct_with_clock(quota, &clock);
        RateLimiterWrapper {
            clock,
            limiter
        }
    }

    pub async fn wait(&self) {
        loop {
            let result = self.limiter.check();
            if result.is_err() {
                let err = result.err().unwrap();
                let wait_time = err.wait_time_from(self.clock.now());
                tokio::time::sleep(wait_time).await;
                continue;
            } else {
                break;
            }
        }
    }
}
