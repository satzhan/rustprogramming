use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
enum Kind { Io, Cpu }

#[derive(Debug)]
struct Task {
    id: u32,
    kind: Kind,
    cpu_cost: u8,
    duration: Duration,
    arrival_time: Instant,
}

impl Task {
    fn new_io(id: u32) -> Self {
        Task { id, kind: Kind::Io, cpu_cost: 10,
            duration: Duration::from_millis(200), arrival_time: Instant::now() }
    }
    fn new_cpu(id: u32) -> Self {
        Task { id, kind: Kind::Cpu, cpu_cost: 35,
            duration: Duration::from_millis(200), arrival_time: Instant::now() }
    }
}

enum HoldState {
    Idle,
    Holding { task_id: u32 },
}

const NUM_WORKERS: usize = 4;
const CPU_CAP: usize = 100;

fn main() { // added cap size and queue logic
    let (gen_tx, gen_rx) = mpsc::channel::<Task>();
    let (work_tx, work_rx) = mpsc::channel::<Task>();
    let work_rx = Arc::new(Mutex::new(work_rx));

    let active_workers = Arc::new(AtomicUsize::new(0));
    let cpu_load = Arc::new(AtomicUsize::new(0));

    let manager_active = Arc::clone(&active_workers);
    let manager_cpu = Arc::clone(&cpu_load);
    let manager = thread::spawn(move || {
        let mut queue: VecDeque<Task> = VecDeque::new();
        let mut generator_done = false;
        let mut hold_state = HoldState::Idle;

        loop {
            match gen_rx.try_recv() {
                Ok(task) => queue.push_back(task),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => generator_done = true,
            }

            if let Some(front) = queue.front() {
                let workers_in_use = manager_active.load(Ordering::Acquire);
                let cpu_in_use = manager_cpu.load(Ordering::Acquire);
                let cost = front.cpu_cost as usize;

                let worker_ok = workers_in_use < NUM_WORKERS;
                let cpu_ok = cpu_in_use + cost <= CPU_CAP;

                if worker_ok && cpu_ok {
                    let task = queue.pop_front().unwrap();

                    if let HoldState::Holding { task_id } = hold_state {
                        if task_id == task.id {
                            println!("    [manager] release task {} (now fits)", task.id);
                        }
                    }

                    println!("    [manager] dispatch task {} ({:?}, +{}%)  [workers {}/{}, cpu {}%]",
                        task.id, task.kind, cost, workers_in_use, NUM_WORKERS, cpu_in_use);
                    work_tx.send(task).unwrap();
                    hold_state = HoldState::Idle;
                } else {
                    let already_holding_this = matches!(
                        hold_state,
                        HoldState::Holding { task_id } if task_id == front.id
                    );
                    if !already_holding_this {
                        let reason = if !worker_ok { "no free worker" } else { "cpu cap" };
                        println!("    [manager] hold task {} ({:?}, +{}%) — {}  [workers {}/{}, cpu {}%]",
                            front.id, front.kind, cost, reason, workers_in_use, NUM_WORKERS, cpu_in_use);
                        hold_state = HoldState::Holding { task_id: front.id };
                    }
                }
            } else {
                hold_state = HoldState::Idle;
            }

            if generator_done && queue.is_empty()
                && manager_active.load(Ordering::Acquire) == 0 {
                drop(work_tx);
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        println!("    [manager] done");
    });

    let mut worker_handles = Vec::new();
    for w in 0..NUM_WORKERS {
        let work_rx = Arc::clone(&work_rx);
        let active = Arc::clone(&active_workers);
        let cpu = Arc::clone(&cpu_load);
        let handle = thread::spawn(move || {
            loop {
                let task = { let rx = work_rx.lock().unwrap(); rx.recv() };
                match task {
                    Ok(task) => {
                        let cost = task.cpu_cost as usize;
                        active.fetch_add(1, Ordering::AcqRel);
                        cpu.fetch_add(cost, Ordering::AcqRel);
                        println!("        [worker {}] start task {} ({:?}), cpu now ~{}%",
                            w, task.id, task.kind, cpu.load(Ordering::Acquire));
                        thread::sleep(task.duration);
                        cpu.fetch_sub(cost, Ordering::AcqRel);
                        active.fetch_sub(1, Ordering::AcqRel);
                    }
                    Err(_) => break,
                }
            }
            println!("        [worker {}] done", w);
        });
        worker_handles.push(handle);
    }

    let mix = [Kind::Cpu, Kind::Cpu, Kind::Cpu, Kind::Io, Kind::Cpu,
               Kind::Io, Kind::Cpu, Kind::Io, Kind::Io, Kind::Cpu];
    for (i, kind) in mix.iter().enumerate() {
        let id = (i + 1) as u32;
        let task = match kind {
            Kind::Io => Task::new_io(id),
            Kind::Cpu => Task::new_cpu(id),
        };
        gen_tx.send(task).unwrap();
        thread::sleep(Duration::from_millis(20));
    }
    drop(gen_tx);

    manager.join().unwrap();
    for h in worker_handles { h.join().unwrap(); }
}