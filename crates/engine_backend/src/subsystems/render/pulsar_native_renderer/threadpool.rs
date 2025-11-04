use std::{sync::{Arc, Mutex, Condvar}, thread};

pub struct ThreadPool { tx: crossbeam_queue::SegQueue<Box<dyn FnOnce() + Send>>, threads: Vec<std::thread::JoinHandle<()>>, cv: Arc<(Mutex<bool>, Condvar)> }

impl ThreadPool {
    pub fn new(n: usize) -> Self {
        let tx = crossbeam_queue::SegQueue::new();
        let cv = Arc::new((Mutex::new(false), Condvar::new()));
        let mut threads = Vec::new();
        for _ in 0..n {
            let q = tx.clone(); let cvc = cv.clone();
            threads.push(thread::spawn(move || loop {
                while let Some(job) = q.pop() { job(); }
                let (lock, c) = (&cvc.0, &cvc.1); let mut pending = lock.lock().unwrap(); *pending = false; let _ = c.wait(pending).unwrap();
            }));
        }
        Self { tx, threads, cv }
    }

    pub fn execute<R: Send + 'static>(&self, f: impl FnOnce() -> R + Send + 'static) -> Task<R> {
        let pair = Arc::new((Mutex::new(None::<R>), Condvar::new()));
        let pair2 = pair.clone();
        self.tx.push(Box::new(move || {
            let r = f();
            let (m, c) = (&pair2.0, &pair2.1); let mut slot = m.lock().unwrap(); *slot = Some(r); c.notify_one();
        }));
        self.cv.1.notify_all();
        Task { pair }
    }
}

pub struct Task<R> { pair: Arc<(Mutex<Option<R>>, Condvar)> }
impl<R> Task<R> { pub fn wait(self) -> R { let (m,c)=(&self.pair.0,&self.pair.1); let mut slot=m.lock().unwrap(); while slot.is_none(){ slot=c.wait(slot).unwrap(); } slot.take().unwrap() } }