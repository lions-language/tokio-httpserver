use tokio::io::AsyncWrite;

use std::collections::HashMap;
use std::future::Future;
use std::pin::{Pin};
use std::marker::Unpin;
use std::sync::{self, Arc};
use std::sync::RwLock;

use crate::tcpstream::route::url_trietree::TrieTree as UrlTree;
use crate::stream::{self, Stream};

use crate::*;

pub type SharedContext = Arc<RwLock<Context>>;

pub struct Context {
    pub data: SendAny
}

#[derive(Clone)]
pub struct RegisterOptions {
}

impl Default for RegisterOptions {
    fn default() -> Self {
        Self {
        }
    }
}

pub struct Data<Writer: tokio::io::AsyncWrite + Send + Unpin> {
    pub creator: FutureCreator<Writer>,
    pub context: Option<SharedContext>,
    pub options: RegisterOptions
}

impl<Writer: AsyncWrite + Send + Unpin> Clone for Data<Writer> {
    fn clone(&self) -> Self {
        Self {
            creator: self.creator.clone(),
            context: self.context.clone(),
            options: self.options.clone()
        }
    }
}

impl<Writer: AsyncWrite + Send + Unpin> Data<Writer> {
    pub fn new(creator: FutureCreator<Writer>
               , context: Option<SharedContext>
               , options: RegisterOptions) -> Self {
        Self {
            creator: creator,
            context: context,
            options: options
        }
    }
}

pub type SharedRoute<Writer: AsyncWrite + Send + Unpin> = Arc<RwLock<Route<Writer>>>;

pub struct Route<Writer: AsyncWrite + Send + Unpin> {
    tree: TrieTree<MethodRoute<Writer>>
}

impl<Writer: AsyncWrite + Send + Unpin> Route<Writer> {
    pub fn register(
        &mut self, path: &[u8], method: Method
        , creator: FutureCreator<Writer>
        , context: Option<SharedContext>
        , options: RegisterOptions) -> Result<()> {
        let node = self.tree.push(path, MethodRoute::new);
        match node {
            Ok(n) => {
                return n.write().unwrap()
                    .data_mut().as_mut().unwrap()
                    .insert(
                        method, Data::new(
                            creator, HeaderRoute::new(), context, options));
            },
            Err(err) => {
                return Err(err.into());
            }
        }
    }

    pub async fn find<S: Stream>(
        &self, stream: &mut S, method: &Method, length: &mut usize) -> Result<Data<Writer>> {
        let node = match self.tree.find_from_stream(stream, length).await {
            Ok(node) => node,
            Err(err) => {
                return Err(err.into());
            }
        };

        let creator = node.read().unwrap().data_ref().as_ref().unwrap().find(method);
        creator
    }

    pub fn new() -> Self {
        Self {
            tree: TrieTree::new()
        }
    }
}


////////////////////////////////
struct MethodRoute<Writer: AsyncWrite + Send + Unpin> {
    handlers: HashMap<Method, Data<Writer>>
}

impl<Writer: AsyncWrite + Send + Unpin> MethodRoute<Writer> {
    fn insert(&mut self, method: Method, data: Data<Writer>) -> Result<()> {
        match self.handlers.get(&method) {
            Some(h) => {
                return Err(Error::Simple(ErrorKind::MethodIsExist));
            },
            None => {
                self.handlers.insert(method, data);
            }
        }

        Ok(())
    }

    fn find(&self, method: &Method) -> Result<Data<Writer>> {
        match self.handlers.get(method) {
            Some(h) => {
                return Ok(h.clone());
            },
            None => {
                return Err(Error::Simple(ErrorKind::RouteIsEmpty));
            }
        }
    }
    
    fn new() -> Self {
        Self {
            handlers: HashMap::new()
        }
    }
}
