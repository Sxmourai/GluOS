use alloc::{collections::BinaryHeap, vec::Vec};
use spin::Mutex;

type Tid = usize;

/// The scheduler for a ThreadPool
pub trait Scheduler: 'static {
    /// Push a thread to the back of ready queue.
    fn push(&self, tid: Tid);
    /// Select a thread to run, pop it from the queue.
    fn pop(&self, cpu_id: usize) -> Option<Tid>;
    /// Got a tick from CPU.
    /// Return true if need reschedule.
    fn tick(&self, current_tid: Tid) -> bool;
    /// Set priority of a thread.
    fn set_priority(&self, tid: Tid, priority: u8);
    /// remove a thread in ready queue.
    fn remove(&self, tid: Tid);
}

fn expand<T: Default + Clone>(vec: &mut Vec<T>, id: usize) {
    let len = vec.len();
    vec.resize(len.max(id + 1), T::default());
}


pub struct O1Scheduler {
    inner: Mutex<O1SchedulerInner>,
}

struct O1SchedulerInner {
    active_queue: usize,
    queues: [Vec<Tid>; 2],
}

impl Scheduler for O1Scheduler {
    fn push(&self, tid: usize) {
        self.inner.lock().push(tid);
    }
    fn pop(&self, _cpu_id: usize) -> Option<usize> {
        self.inner.lock().pop()
    }
    fn tick(&self, current_tid: usize) -> bool {
        self.inner.lock().tick(current_tid)
    }
    fn set_priority(&self, _tid: usize, _priority: u8) {}
    fn remove(&self, _tid: usize) {
        unimplemented!()
    }
}

impl O1Scheduler {
    pub fn new() -> Self {
        let inner = O1SchedulerInner {
            active_queue: 0,
            queues: [Vec::new(), Vec::new()],
        };
        O1Scheduler {
            inner: Mutex::new(inner),
        }
    }
}

impl O1SchedulerInner {
    fn push(&mut self, tid: Tid) {
        let inactive_queue = 1 - self.active_queue;
        self.queues[inactive_queue].push(tid);
        serial_println!("o1 push {}", tid - 1);
    }

    fn pop(&mut self) -> Option<Tid> {
        let ret = match self.queues[self.active_queue].pop() {
            Some(tid) => return Some(tid),
            None => {
                // active queue is empty, swap 'em
                self.active_queue = 1 - self.active_queue;
                self.queues[self.active_queue].pop()
            }
        };
        serial_println!("o1 pop {:?}", ret);
        ret
    }

    fn tick(&mut self, _current: Tid) -> bool {
        true
    }
}


pub struct RRScheduler {
    inner: Mutex<RRSchedulerInner>,
}

struct RRSchedulerInner {
    max_time_slice: usize,
    infos: Vec<RRProcInfo>,
}

#[derive(Debug, Default, Copy, Clone)]
struct RRProcInfo {
    present: bool,
    rest_slice: usize,
    prev: Tid,
    next: Tid,
}

impl Scheduler for RRScheduler {
    fn push(&self, tid: usize) {
        self.inner.lock().push(tid);
    }
    fn pop(&self, _cpu_id: usize) -> Option<usize> {
        self.inner.lock().pop()
    }
    fn tick(&self, current_tid: usize) -> bool {
        self.inner.lock().tick(current_tid)
    }
    fn set_priority(&self, _tid: usize, _priority: u8) {}
    fn remove(&self, tid: usize) {
        self.inner.lock().remove(tid)
    }
}

impl RRScheduler {
    pub fn new(max_time_slice: usize) -> Self {
        let inner = RRSchedulerInner {
            max_time_slice,
            infos: Vec::default(),
        };
        RRScheduler {
            inner: Mutex::new(inner),
        }
    }
}

impl RRSchedulerInner {
    fn push(&mut self, tid: Tid) {
        let tid = tid + 1;
        expand(&mut self.infos, tid);
        {
            let info = &mut self.infos[tid];
            assert!(!info.present);
            info.present = true;
            if info.rest_slice == 0 {
                info.rest_slice = self.max_time_slice;
            }
        }
        self._list_add_before(tid, 0);
        serial_println!("rr push {}", tid - 1);
    }

    fn pop(&mut self) -> Option<Tid> {
        let ret = match self.infos[0].next {
            0 => None,
            tid => {
                self.infos[tid].present = false;
                self._list_remove(tid);
                Some(tid - 1)
            }
        };
        serial_println!("rr pop {:?}", ret);
        ret
    }

    fn tick(&mut self, current: Tid) -> bool {
        let current = current + 1;
        expand(&mut self.infos, current);
        assert!(!self.infos[current].present);

        let rest = &mut self.infos[current].rest_slice;
        if *rest > 0 {
            *rest -= 1;
        } else {
            warn!("current process rest_slice = 0, need reschedule")
        }
        *rest == 0
    }

    fn remove(&mut self, tid: Tid) {
        self._list_remove(tid + 1);
        self.infos[tid + 1].present = false;
    }
}

impl RRSchedulerInner {
    fn _list_add_before(&mut self, i: Tid, at: Tid) {
        let prev = self.infos[at].prev;
        self.infos[i].next = at;
        self.infos[i].prev = prev;
        self.infos[prev].next = i;
        self.infos[at].prev = i;
    }
    fn _list_add_after(&mut self, i: Tid, at: Tid) {
        let next = self.infos[at].next;
        self._list_add_before(i, next);
    }
    fn _list_remove(&mut self, i: Tid) {
        let next = self.infos[i].next;
        let prev = self.infos[i].prev;
        self.infos[next].prev = prev;
        self.infos[prev].next = next;
        self.infos[i].next = 0;
        self.infos[i].prev = 0;
    }
}


use core::cmp::{Ordering, Reverse};

pub struct StrideScheduler {
    inner: Mutex<StrideSchedulerInner>,
}

pub struct StrideSchedulerInner {
    max_time_slice: usize,
    infos: Vec<StrideProcInfo>,
    queue: BinaryHeap<Reverse<(Stride, Tid)>>, // It's max heap, so use Reverse
}

#[derive(Debug, Default, Copy, Clone)]
struct StrideProcInfo {
    present: bool,
    rest_slice: usize,
    stride: Stride,
    priority: u8,
}

const BIG_STRIDE: Stride = Stride(0x7FFFFFFF);

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
struct Stride(u32);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Stride {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0 == other.0 {
            Ordering::Equal
        } else {
            let sub = other.0.overflowing_sub(self.0).0;
            if sub <= BIG_STRIDE.0 {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
    }
}

impl StrideProcInfo {
    fn pass(&mut self) {
        let pass = if self.priority == 0 {
            BIG_STRIDE.0
        } else {
            BIG_STRIDE.0 / self.priority as u32
        };
        self.stride = Stride(self.stride.0.overflowing_add(pass).0);
    }
}

impl Scheduler for StrideScheduler {
    fn push(&self, tid: usize) {
        self.inner.lock().push(tid);
    }
    fn pop(&self, _cpu_id: usize) -> Option<usize> {
        self.inner.lock().pop()
    }
    fn tick(&self, current_tid: usize) -> bool {
        self.inner.lock().tick(current_tid)
    }
    fn set_priority(&self, tid: usize, priority: u8) {
        self.inner.lock().set_priority(tid, priority);
    }
    fn remove(&self, tid: usize) {
        self.inner.lock().remove(tid);
    }
}

impl StrideScheduler {
    pub fn new(max_time_slice: usize) -> Self {
        let inner = StrideSchedulerInner {
            max_time_slice,
            infos: Vec::default(),
            queue: BinaryHeap::default(),
        };
        StrideScheduler {
            inner: Mutex::new(inner),
        }
    }
}

impl StrideSchedulerInner {
    fn push(&mut self, tid: Tid) {
        expand(&mut self.infos, tid);
        let info = &mut self.infos[tid];
        info.present = true;
        if info.rest_slice == 0 {
            info.rest_slice = self.max_time_slice;
        }
        self.queue.push(Reverse((info.stride, tid)));
        serial_println!("stride push {}", tid);
    }

    fn pop(&mut self) -> Option<Tid> {
        let ret = self.queue.pop().map(|Reverse((_, tid))| tid);
        if let Some(tid) = ret {
            let info = &mut self.infos[tid];
            if !info.present {
                return self.pop();
            }
            let old_stride = info.stride;
            info.pass();
            let stride = info.stride;
            info.present = false;
            serial_println!("stride {} {:#x} -> {:#x}", tid, old_stride.0, stride.0);
        }
        serial_println!("stride pop {:?}", ret);
        ret
    }

    fn tick(&mut self, current: Tid) -> bool {
        expand(&mut self.infos, current);
        assert!(!self.infos[current].present);

        let rest = &mut self.infos[current].rest_slice;
        if *rest > 0 {
            *rest -= 1;
        } else {
            warn!("current process rest_slice = 0, need reschedule")
        }
        *rest == 0
    }

    fn set_priority(&mut self, tid: Tid, priority: u8) {
        self.infos[tid].priority = priority;
        serial_println!("stride {} priority = {}", tid, priority);
    }

    fn remove(&mut self, tid: Tid) {
        self.infos[tid].present = false;
    }
}


use deque::{self, Stealer, Stolen, Worker};

use crate::serial_println;

pub struct WorkStealingScheduler {
    /// The ready queue of each processors
    workers: Vec<Worker<Tid>>,
    /// Stealers to all processors' queue
    stealers: Vec<Stealer<Tid>>,
}

impl WorkStealingScheduler {
    pub fn new(core_num: usize) -> Self {
        let (workers, stealers) = (0..core_num).map(|_| deque::new()).unzip();
        WorkStealingScheduler { workers, stealers }
    }
}

impl Scheduler for WorkStealingScheduler {
    fn push(&self, tid: usize) {
        // not random, but uniform
        // no sync, because we don't need to
        static mut WORKER_CPU: usize = 0;
        let n = self.workers.len();
        let mut cpu = unsafe {
            WORKER_CPU = WORKER_CPU + 1;
            if WORKER_CPU >= n {
                WORKER_CPU -= n;
            }
            WORKER_CPU
        };

        // potential racing, so we just check once more
        if cpu >= n {
            cpu -= n;
        }
        self.workers[cpu].push(tid);
        serial_println!("work-stealing: cpu{} push thread {}", cpu, tid);
    }

    fn pop(&self, cpu_id: usize) -> Option<usize> {
        if let Some(tid) = self.workers[cpu_id].pop() {
            serial_println!("work-stealing: cpu{} pop thread {}", cpu_id, tid);
            return Some(tid);
        }
        let n = self.workers.len();
        for i in 1..n {
            let mut other_id = cpu_id + i;
            if other_id >= n {
                other_id -= n;
            }
            loop {
                match self.stealers[other_id].steal() {
                    Stolen::Abort => {} // retry
                    Stolen::Empty => break,
                    Stolen::Data(tid) => {
                        serial_println!(
                            "work-stealing: cpu{} steal thread {} from cpu{}",
                            cpu_id,
                            tid,
                            other_id
                        );
                        return Some(tid);
                    }
                }
            }
        }
        None
    }

    fn tick(&self, _current_tid: usize) -> bool {
        true
    }

    fn set_priority(&self, _tid: usize, _priority: u8) {}

    fn remove(&self, _tid: usize) {}
}