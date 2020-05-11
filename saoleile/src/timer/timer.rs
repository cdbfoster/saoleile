use std::collections::HashMap;
use std::sync::{mpsc, Mutex, Weak};
use std::time::{Duration, Instant};

use crate::timer::TimerData;

pub struct Wakeup {
    pub time: Instant,
    pub timer: Timer,
}

pub struct Timer {
    pub(super) id: usize,
    pub(super) original: bool,
    pub(super) timers: Weak<Mutex<HashMap<usize, TimerData>>>,
    pub(super) update_sender: Weak<Mutex<mpsc::Sender<bool>>>,
}

impl Timer {
    pub fn start(&mut self) {
        let timers = self.timers.upgrade().expect("Cannot access timer data");
        {
            let mut guard = timers.lock().unwrap();
            let mut timer_data = guard.get_mut(&self.id).unwrap();

            let now = Instant::now();

            timer_data.active = true;
            timer_data.start_time = now;
            timer_data.next_tick = now + timer_data.interval;
        }
        let update_sender = self.update_sender.upgrade().expect("Cannot access timing thread waker");
        update_sender.lock().unwrap().send(false).expect("Cannot wake timing thread");
    }

    pub fn stop(&mut self) {
        let timers = self.timers.upgrade().expect("Cannot access timer data");
        let mut guard = timers.lock().unwrap();
        guard.get_mut(&self.id).unwrap().active = false;
    }

    pub fn set_interval(&mut self, interval: Duration) {
        let timers = self.timers.upgrade().expect("Cannot access timer data");
        let mut guard = timers.lock().unwrap();
        guard.get_mut(&self.id).unwrap().interval = interval;
    }

    pub fn get_interval(&self) -> Duration {
        let timers = self.timers.upgrade().expect("Cannot access timer data");
        let guard = timers.lock().unwrap();
        guard.get(&self.id).unwrap().interval
    }

    pub fn get_next_tick(&self) -> Instant {
        let timers = self.timers.upgrade().expect("Cannot access timer data");
        let guard = timers.lock().unwrap();
        guard.get(&self.id).unwrap().next_tick
    }

    pub fn get_running_time(&self) -> Duration {
        let timers = self.timers.upgrade().expect("Cannot access timer data");
        let guard = timers.lock().unwrap();
        let timer_data = guard.get(&self.id).unwrap();

        if timer_data.active {
            timer_data.start_time.elapsed()
        } else {
            Duration::from_secs(0)
        }
    }

    pub fn is_active(&self) -> bool {
        let timers = self.timers.upgrade().expect("Cannot access timer data");
        let guard = timers.lock().unwrap();
        guard.get(&self.id).unwrap().active
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if self.original {
            if let Some(timers) = self.timers.upgrade() {
                let mut guard = timers.lock().unwrap();
                guard.remove(&self.id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;
    use std::thread;

    use crate::timer::TimerManager;

    #[test]
    fn test_timer_start_stop() {
        let tm = TimerManager::new();

        let counter = Arc::new(Mutex::new(0));
        let mut t = tm.timer(Duration::from_secs(1) / 10, {
            let counter = counter.clone();
            move |_| *counter.lock().unwrap() += 1
        });

        thread::sleep(Duration::from_millis(1010));

        t.stop();
        thread::sleep(Duration::from_millis(1000));
        t.start();

        thread::sleep(Duration::from_millis(1010));
        assert_eq!(*counter.lock().unwrap(), 20);
    }

    #[test]
    fn test_timer_change_interval() {
        let tm = TimerManager::new();

        let counter = Arc::new(Mutex::new(0));
        let mut t = tm.timer(Duration::from_secs(1) / 10, {
            let counter = counter.clone();
            move |_| *counter.lock().unwrap() += 1
        });

        thread::sleep(Duration::from_millis(950));
        t.set_interval(Duration::from_secs(1) / 20);
        thread::sleep(Duration::from_millis(50 + 1010));

        assert_eq!(*counter.lock().unwrap(), 30);
    }
}