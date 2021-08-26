pub(crate) mod opt;

pub type SharedContext = Arc<RwLock<Context>>;

pub struct Context {
    pub data: SendAny
}

pub type FutureExecutor = Pin<Box<Future<Output = ()> + Send + 'static>>;
pub type FutureCreator<Writer: tokio::io::AsyncWrite + Send + Unpin>
    = fn(Option<SharedContext>, Request, Response<Writer>) -> FutureExecutor;

pub type BodyBuilder<'a> = Pin<Box<Future<Output = Result<SendAny>> + 'a>>;
pub type BodyBuilderCreator<'a, S: stream::Stream> = fn(&'a mut S, &'a Box<RequestHeader>) -> BodyBuilder<'a>;

