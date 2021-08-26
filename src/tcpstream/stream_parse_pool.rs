use tokio::{self, task, net, sync::RwLock, sync::mpsc};
use tokio::net::TcpStream as TokioTcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use std::future::Future;
use std::sync::{Arc};
use std::pin::Pin;

use crate::stream::tcp::Stream as TcpStream;
use crate::route::url::{SharedRoute, FutureExecutor};

use crate::taskpool::future::Pool as FuturePool;

use crate::*;

pub type HandlePool = Arc<RwLock<FuturePool<FutureExecutor>>>;

pub type Creator = fn(TokioTcpStream, SharedRoute<OwnedWriteHalf>, HandlePool) -> Pin<Box<Future<Output = ()>>>;

struct Item {
    creator: Creator,
    stream: TokioTcpStream,
    route: SharedRoute<OwnedWriteHalf>,
    handle_pool: HandlePool
}

pub struct Pool {
    sender: mpsc::Sender<Item>
}

impl Pool {
    pub async fn execute(
        &mut self, creator: Creator, stream: TokioTcpStream
        , route: SharedRoute<OwnedWriteHalf>, handle_pool: HandlePool) {
        self.sender.send(Item{
            creator: creator,
            stream: stream,
            route: route,
            handle_pool: handle_pool
        }).await;
    }

    fn run_thread(receiver: Arc<RwLock<mpsc::Receiver<Item>>>) {
        std::thread::spawn(|| {
            let rt  = tokio::runtime::Runtime::new().unwrap();

            rt.block_on(async {
                let local = task::LocalSet::new();

                local.run_until(async move {
                    loop {
                        let mut receiver = receiver.write().await;

                        let item = match receiver.recv().await {
                            Some(item) => item,
                            None => {
                                return;
                            }
                        };

                        let executor = (item.creator)(item.stream, item.route, item.handle_pool);

                        task::spawn_local(executor);

                        task::yield_now().await;
                    }
                }).await;
            });
        });
    }

    pub fn new(pool_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel::<Item>(1);
        let mut receiver = Arc::new(RwLock::new(receiver));
        
        for _ in 0..pool_size {
            Pool::run_thread(receiver.clone());
        }

        Self {
            sender: sender,
        }
    }
}

