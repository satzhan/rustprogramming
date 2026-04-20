use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TaskKind {
    Cpu,
    Io,
}

#[derive(Debug, Clone)]
struct Task {
    id: usize,
    kind: TaskKind,
    cpu: i32,
    arrival_time: Instant,
    enqueue_time: Instant,
    duration: Duration,
}

#[derive(Debug, Clone)]
enum MetricEvent {
    Enqueued {
        task_id: usize,
        kind: TaskKind,
    },
    Started {
        worker_id: usize,
        task_id: usize,
        kind: TaskKind,
        arrival_time: Instant,
        enqueue_time: Instant,
        start_time: Instant,
    },
    Finished {
        worker_id: usize,
        task_id: usize,
        kind: TaskKind,
        arrival_time: Instant,
        start_time: Instant,
        finish_time: Instant,
    },
}

#[derive(Default)]
struct Stats {
    total_enqueued: usize,
    total_started: usize,
    total_finished: usize,

    cpu_finished: usize,
    io_finished: usize,

    queue_depth: usize,
    max_queue_depth: usize,

    sum_wait: Duration,
    max_wait: Duration,

    sum_turnaround: Duration,

    started_at: Option<Instant>,
    finished_at: Option<Instant>,

    busy_by_worker: HashMap<usize, Duration>,
    tasks_by_worker: HashMap<usize, usize>,

    queued_kind: HashMap<TaskKind, usize>,

    last_task_id_seen: Option<usize>,
}

impl Stats {
    fn apply(&mut self, event: MetricEvent) {
        match event {
            MetricEvent::Enqueued { task_id, kind } => {
                self.total_enqueued += 1;
                self.queue_depth += 1;
                self.max_queue_depth = self.max_queue_depth.max(self.queue_depth);
                *self.queued_kind.entry(kind).or_insert(0) += 1;
                self.last_task_id_seen = Some(task_id);
            }

            MetricEvent::Started {
                worker_id: _,
                task_id,
                kind,
                arrival_time,
                enqueue_time,
                start_time,
            } => {
                self.total_started += 1;
                self.queue_depth = self.queue_depth.saturating_sub(1);

                let wait = start_time.duration_since(enqueue_time);
                self.sum_wait += wait;
                self.max_wait = self.max_wait.max(wait);

                self.started_at = Some(match self.started_at {
                    Some(old) => old.min(arrival_time),
                    None => arrival_time,
                });

                if let Some(cnt) = self.queued_kind.get_mut(&kind) {
                    *cnt = cnt.saturating_sub(1);
                }

                self.last_task_id_seen = Some(task_id);
            }

            MetricEvent::Finished {
                worker_id,
                task_id,
                kind,
                arrival_time,
                start_time,
                finish_time,
            } => {
                self.total_finished += 1;

                match kind {
                    TaskKind::Cpu => self.cpu_finished += 1,
                    TaskKind::Io => self.io_finished += 1,
                }

                let turnaround = finish_time.duration_since(arrival_time);
                let busy = finish_time.duration_since(start_time);

                self.sum_turnaround += turnaround;
                *self.busy_by_worker.entry(worker_id).or_insert(Duration::ZERO) += busy;
                *self.tasks_by_worker.entry(worker_id).or_insert(0) += 1;

                self.finished_at = Some(match self.finished_at {
                    Some(old) => old.max(finish_time),
                    None => finish_time,
                });

                self.last_task_id_seen = Some(task_id);
            }
        }
    }

    fn print_snapshot(&self) {
        println!(
            "[monitor] enq={} start={} done={} queue_now={} max_queue={} queued_cpu={} queued_io={} last_task={:?}",
            self.total_enqueued,
            self.total_started,
            self.total_finished,
            self.queue_depth,
            self.max_queue_depth,
            self.queued_kind.get(&TaskKind::Cpu).copied().unwrap_or(0),
            self.queued_kind.get(&TaskKind::Io).copied().unwrap_or(0),
            self.last_task_id_seen,
        );
    }

    fn print_final(&self, workers: usize) {
        println!("\n===== FINAL METRICS =====");
        println!("Total enqueued      : {}", self.total_enqueued);
        println!("Total completed     : {}", self.total_finished);
        println!("CPU completed       : {}", self.cpu_finished);
        println!("IO completed        : {}", self.io_finished);
        println!("Max queue depth     : {}", self.max_queue_depth);
        println!("Max wait time       : {:.2} ms", ms(self.max_wait));

        if self.total_finished > 0 {
            println!(
                "Average wait        : {:.2} ms",
                ms(self.sum_wait) / self.total_finished as f64
            );
            println!(
                "Average turnaround  : {:.2} ms",
                ms(self.sum_turnaround) / self.total_finished as f64
            );
        }

        if let (Some(start), Some(end)) = (self.started_at, self.finished_at) {
            let makespan = end.duration_since(start);
            println!("Makespan            : {:.2} ms", ms(makespan));

            for worker_id in 0..workers {
                let busy = self
                    .busy_by_worker
                    .get(&worker_id)
                    .copied()
                    .unwrap_or(Duration::ZERO);
                let count = self.tasks_by_worker.get(&worker_id).copied().unwrap_or(0);

                let util = if makespan.is_zero() {
                    0.0
                } else {
                    100.0 * busy.as_secs_f64() / makespan.as_secs_f64()
                };

                println!(
                    "Worker {worker_id:>2}: tasks={count:>2}, busy={:>7.2} ms, util={:>6.2}%",
                    ms(busy),
                    util
                );
            }
        }
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Deterministic workload: no external crates needed.
/// Returns (kind, duration, gap_before_arrival)
fn build_task_plan(count: usize) -> Vec<(TaskKind, Duration, Duration)> {
    let cpu_durations = [180_u64, 220, 260, 300];
    let io_durations = [50_u64, 70, 90, 110];

    let mut plan = Vec::with_capacity(count);

    for i in 0..count {
        let kind = if i % 3 == 0 {
            TaskKind::Cpu
        } else {
            TaskKind::Io
        };

        let duration = match kind {
            TaskKind::Cpu => Duration::from_millis(cpu_durations[i % cpu_durations.len()]),
            TaskKind::Io => Duration::from_millis(io_durations[i % io_durations.len()]),
        };

        // A few bursts mixed with normal gaps
        let gap = if i % 8 == 0 {
            Duration::from_millis(0)
        } else if i % 5 == 0 {
            Duration::from_millis(25)
        } else {
            Duration::from_millis(40)
        };

        plan.push((kind, duration, gap));
    }

    plan
}

fn main() {
    const WORKERS: usize = 4;
    const TASKS: usize = 24;

    let (task_tx, task_rx) = mpsc::channel::<Task>();
    let (metric_tx, metric_rx) = mpsc::channel::<MetricEvent>();

    // std::sync::mpsc::Receiver cannot be cloned, so for a simple worker pool
    // we wrap it in Arc<Mutex<_>>. Only the "take next task" step is serialized.
    let shared_rx = Arc::new(Mutex::new(task_rx));

    // Monitor thread: the only thread allowed to own and mutate Stats.
    let monitor_handle = {
        thread::spawn(move || {
            let mut stats = Stats::default();

            loop {
                match metric_rx.recv_timeout(Duration::from_millis(300)) {
                    Ok(event) => stats.apply(event),

                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        stats.print_snapshot();
                    }

                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        stats.print_final(WORKERS);
                        break;
                    }
                }
            }
        })
    };

    // Worker pool
    let mut worker_handles = Vec::new();

    for worker_id in 0..WORKERS {
        let rx = Arc::clone(&shared_rx);
        let tx = metric_tx.clone();

        let handle = thread::spawn(move || {
            loop {
                let task = {
                    let receiver = rx.lock().unwrap();
                    receiver.recv()
                };

                let task = match task {
                    Ok(task) => task,
                    Err(_) => break, // clean shutdown
                };

                let start_time = Instant::now();

                tx.send(MetricEvent::Started {
                    worker_id,
                    task_id: task.id,
                    kind: task.kind,
                    arrival_time: task.arrival_time,
                    enqueue_time: task.enqueue_time,
                    start_time,
                })
                .unwrap();

                thread::sleep(task.duration);

                let finish_time = Instant::now();

                tx.send(MetricEvent::Finished {
                    worker_id,
                    task_id: task.id,
                    kind: task.kind,
                    arrival_time: task.arrival_time,
                    start_time,
                    finish_time,
                })
                .unwrap();
            }
        });

        worker_handles.push(handle);
    }

    // Generator thread
    let generator_handle = {
        let tx_tasks = task_tx.clone();
        let tx_metrics = metric_tx.clone();

        thread::spawn(move || {
            let plan = build_task_plan(TASKS);
            let zero = Instant::now();

            for (id, (kind, duration, gap)) in plan.into_iter().enumerate() {
                thread::sleep(gap);

                let now = Instant::now();
                let task = Task {
                    id,
                    kind,
                    arrival_time: now,
                    enqueue_time: now,
                    duration,
                };

                tx_metrics
                    .send(MetricEvent::Enqueued { task_id: id, kind })
                    .unwrap();

                tx_tasks.send(task).unwrap();

                println!(
                    "[generator +{:>5.1} ms] queued task {:>2} {:?} ({:>5.1} ms)",
                    ms(now.duration_since(zero)),
                    id,
                    kind,
                    ms(duration)
                );
            }
        })
    };

    // Main no longer needs these original senders.
    drop(task_tx);
    drop(metric_tx);

    generator_handle.join().unwrap();

    for handle in worker_handles {
        handle.join().unwrap();
    }

    monitor_handle.join().unwrap();
}