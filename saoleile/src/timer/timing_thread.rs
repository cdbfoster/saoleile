use std::cmp;
use std::collections::HashMap;
use std::sync::{Arc, mpsc, Mutex};
use std::time::{Duration, Instant};

use crate::timer::TimerData;
use crate::timer::timer::{Timer, Wakeup};

pub(super) fn timer_manager_timing_thread(
    timers: Arc<Mutex<HashMap<usize, TimerData>>>,
    update_sender: Arc<Mutex<mpsc::Sender<bool>>>,
    update_receiver: mpsc::Receiver<bool>,
) {
    loop {
        let sleep_time = {
            let mut guard = timers.lock().unwrap();

            if guard.values().any(|t| t.active) {
                let now = Instant::now();

                // Wake expired timers
                for (&id, timer_data) in guard.iter_mut().filter(|(_, t)| t.active && t.next_tick <= now) {
                    timer_data.next_tick += timer_data.interval;
                    let wakeup = Wakeup {
                        time: now,
                        timer: Timer {
                            id,
                            original: false,
                            timers: Arc::downgrade(&timers),
                            update_sender: Arc::downgrade(&update_sender),
                        },
                    };
                    (timer_data.wake)(wakeup);
                }

                let next_tick = get_next_tick(now, &*guard);

                (next_tick - now) / 2
            } else {
                Duration::from_secs(1)
            }
        };

        if let Ok(quit) = update_receiver.recv_timeout(sleep_time) {
            if quit {
                break;
            }
        }
    }
}

fn get_next_tick(now: Instant, timers: &HashMap<usize, TimerData>) -> Instant {
    let earliest_tick = timers.values().filter_map(|t| if t.active {
        Some(t.next_tick)
    } else {
        None
    }).min().unwrap();
    cmp::max(earliest_tick, now)
}