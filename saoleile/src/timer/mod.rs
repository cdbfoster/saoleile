use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::util::InnerThread;

use self::timing_thread::timer_manager_timing_thread;

pub use self::timer::{Timer, Wakeup};

#[derive(Debug)]
pub struct TimerManager {
    timer_count: Mutex<usize>,
    timers: Arc<Mutex<HashMap<usize, TimerData>>>,
    update_sender: Arc<Mutex<mpsc::Sender<bool>>>, // This has to be shared right now because of https://github.com/rust-lang/rust/issues/39364

    timing_thread: InnerThread,
    running: Mutex<bool>,
}

impl TimerManager {
    pub fn new() -> Self {
        let timers = Arc::new(Mutex::new(HashMap::new()));

        let (sender, update_receiver) = mpsc::channel();
        let update_sender = Arc::new(Mutex::new(sender));

        let timing_thread = InnerThread::new({
            let timers = timers.clone();
            let update_sender = update_sender.clone();
            thread::Builder::new()
                .name("TimerManager timing thread".to_string())
                .spawn(move || timer_manager_timing_thread(timers, update_sender, update_receiver)).unwrap()
        });

        Self {
            timer_count: Mutex::new(0),
            timers,
            update_sender,
            timing_thread,
            running: Mutex::new(true),
        }
    }

    pub fn timer<F>(&self, interval: Duration, wake: F) -> Timer
    where
        F: Fn(Wakeup) + Send + 'static,
    {
        self.timer_aligned(Instant::now(), interval, wake)
    }

    pub fn timer_aligned<F>(&self, aligned_to: Instant, interval: Duration, wake: F) -> Timer
    where
        F: Fn(Wakeup) + Send + 'static,
    {
        assert!(interval > Duration::from_secs(0), "Cannot have a zero interval");

        let now = Instant::now();

        let next_tick = {
            let mut time = aligned_to;
            if time > now {
                while time > now {
                    time -= interval;
                }
                time += interval;
            } else {
                while time < now {
                    time += interval;
                }
            }
            time
        };

        let timer_data = TimerData {
            active: true,
            start_time: now,
            next_tick,
            interval,
            wake: Box::new(wake),
        };

        let id = {
            let mut guard = self.timer_count.lock().unwrap();
            let id = *guard;
            *guard += 1;
            id
        };

        {
            let mut guard = self.timers.lock().unwrap();
            guard.insert(id, timer_data);
        }

        {
            let guard = self.update_sender.lock().unwrap();
            guard.send(false).expect("Could not wake timing thread");
        }

        Timer {
            id,
            original: true,
            timers: Arc::downgrade(&self.timers),
            update_sender: Arc::downgrade(&self.update_sender),
        }
    }

    pub fn shutdown(&self) {
        let mut running = self.running.lock().unwrap();
        if *running {
            self.update_sender.lock().unwrap().send(true).expect("Could not send shutdown message to timing thread");
            self.timing_thread.join();
            *running = false;
        }
    }
}

struct TimerData {
    active: bool,
    start_time: Instant,
    next_tick: Instant,
    interval: Duration,
    wake: Box<dyn Fn(Wakeup) + Send>,
}

impl fmt::Debug for TimerData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TimerData")
            .field("active", &self.active)
            .field("start_time", &self.start_time)
            .field("next_tick", &self.next_tick)
            .field("interval", &self.interval)
            .finish()
    }
}

mod timer;
mod timing_thread;

#[cfg(test)]
mod tests {
    use super::*;

    fn get_error(elapsed: Duration, target: Duration) -> Duration {
        if elapsed > target {
            elapsed - target
        } else {
            target - elapsed
        }
    }

    #[test]
    fn test_fast_timer_precision() {
        const TARGET: Duration = Duration::from_secs(2);
        const FPS: u32 = 240;

        let tm = TimerManager::new();

        let (s, r) = mpsc::channel();
        let _t = tm.timer(Duration::from_secs(1) / FPS, move |_| s.send(()).expect("Can't wake timer"));

        let start = Instant::now();
        for _ in 0..(TARGET.as_secs_f32() * FPS as f32) as usize {
            r.recv().expect("Can't receive wake");
        }

        let elapsed = start.elapsed();
        let error = get_error(elapsed, TARGET);
        assert!(error < Duration::from_millis(1), "Error was greater than or equal to one millisecond: {}ns", error.as_nanos());
    }

    #[test]
    fn test_multi_timer_precision() {
        const TARGET: Duration = Duration::from_secs(2);
        const BASE_FPS: u32 = 120;

        let tm = Arc::new(TimerManager::new());

        let handles = (1..10).map(|i| {
            let tm = tm.clone();
            thread::spawn(move || {
                let fps = BASE_FPS / i;
                let interval = Duration::from_nanos(1_000_000_000 / fps as u64);

                let (s, r) = mpsc::channel();
                let _t = tm.timer(interval, move |_| s.send(()).expect("Can't wake timer"));

                let start = Instant::now();
                for _ in 0..(TARGET.as_secs_f32() * fps as f32) as usize {
                    r.recv().expect("Can't receive wake");
                }

                let elapsed = start.elapsed();
                let error = get_error(elapsed, TARGET);
                (fps, error)
            })
        }).collect::<Vec<_>>();

        for handle in handles {
            let (fps, error) = handle.join().unwrap();
            assert!(error < Duration::from_millis(1), "Error was greater than or equal to one millisecond at {} fps: {}ns", fps, error.as_nanos());
        }
    }
}