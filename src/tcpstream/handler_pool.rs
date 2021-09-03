use tokio::{self, task, net, sync, sync::mpsc};
use std::future::Future;
use std::sync::Arc;

pub struct ExecutePool<T>
    where T: Future + Send + 'static,
          T::Output: Send + 'static {
    sender: mpsc::Sender<T>
}

impl<T> ExecutePool<T>
    where T: Future + Send + 'static,
          T::Output: Send + 'static {

    pub async fn execute(&mut self, f: T) {
        self.sender.send(f).await;
    }

    fn run_thread(receiver: Arc<sync::Mutex<mpsc::Receiver<T>>>) {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();

            rt.block_on(async {
                let local = task::LocalSet::new();

                local.run_until(async move {
                    loop {
                        let mut receiver = receiver.lock().await;

                        let executor = match receiver.recv().await {
                            Some(e) => e,
                            None => {
                                return;
                            }
                        };

                        task::spawn_local(executor);

                        task::yield_now().await;
                    }
                }).await;
            });
        });
    }

    pub fn new(pool_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel(1);
        let mut receiver = Arc::new(sync::Mutex::new(receiver));
        
        for _ in 0..pool_size {
            Pool::run_thread(receiver.clone());
        }

        Self {
            sender: sender,
        }
    }
}

