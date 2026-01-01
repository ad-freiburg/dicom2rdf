use log::info;
use std::sync::mpsc;

pub struct ProgressSender {
    counter: usize,
    threshold: usize,
    tx: mpsc::Sender<usize>,
}

impl Clone for ProgressSender {
    fn clone(&self) -> Self {
        Self {
            counter: 0,
            threshold: self.threshold,
            tx: self.tx.clone(),
        }
    }
}

impl ProgressSender {
    fn new(tx: mpsc::Sender<usize>, threshold: usize) -> Self {
        Self {
            counter: 0,
            threshold,
            tx,
        }
    }

    pub fn tick(&mut self) {
        self.counter += 1;
        if self.counter >= self.threshold {
            let _ = self.tx.send(self.counter);
            self.counter = 0;
        }
    }
}

impl Drop for ProgressSender {
    fn drop(&mut self) {
        if self.counter > 0 {
            let _ = self.tx.send(self.counter);
        }
    }
}

pub fn progress_logger() -> (ProgressSender, std::thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel();
    let now = std::time::Instant::now();
    let threshold = 10000;
    let thread = std::thread::spawn(move || {
        let mut next_milestone = threshold;
        let mut total = 0;
        while let Ok(x) = rx.recv() {
            total += x;
            if total >= next_milestone {
                info!("{} files converted", next_milestone);
                next_milestone += threshold;
            }
        }
        let elapsed = now.elapsed();
        let files_per_second = total as f64 / elapsed.as_secs_f64();
        info!(
            "\x1b[1mFinished converting {} files in {:.2?} ({:.2} files/s)\x1b[0m",
            total, elapsed, files_per_second
        );
    });
    (
        ProgressSender::new(tx, threshold / rayon::current_num_threads()),
        thread,
    )
}
