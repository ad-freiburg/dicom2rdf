use log::info;
use std::sync::mpsc;

pub fn mk_progress_logger(progress_rx: mpsc::Receiver<()>) -> std::thread::JoinHandle<()> {
    let now = std::time::Instant::now();
    std::thread::spawn(move || {
        let mut total = 0;
        while let Ok(()) = progress_rx.recv() {
            total += 1;
            if total % 10000 == 0 {
                info!("{} files converted", total);
            }
        }
        let elapsed = now.elapsed();
        let files_per_second = total as f64 / elapsed.as_secs_f64();
        info!(
            "\x1b[1mFinished converting {} files in {:.2?} ({:.2} files/s)\x1b[0m",
            total, elapsed, files_per_second
        );
    })
}
