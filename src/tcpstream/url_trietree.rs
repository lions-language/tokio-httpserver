use std::collections;
use std::sync;

use crate::*;
use crate::tcpstream::Stream;

pub struct TrieNode<T> {
    data: Option<T>,
    nodes: collections::HashMap<Option<u8>, sync::Arc<sync::RwLock<TrieNode<T>>>>
}

impl<T> TrieNode<T> {
    fn new_node(&self) -> sync::Arc<sync::RwLock<TrieNode<T>>> {
        sync::Arc::new(sync::RwLock::new(TrieNode::new(None)))
    }

    fn insert_node(&mut self, u: u8, node: sync::Arc<sync::RwLock<TrieNode<T>>>) {
        self.nodes.insert(Some(u), node);
    }

    fn new_none(&self, t: T) -> sync::Arc<sync::RwLock<TrieNode<T>>> {
        sync::Arc::new(sync::RwLock::new(TrieNode::new(Some(t))))
    }

    fn insert_none(&mut self, node: sync::Arc<sync::RwLock<TrieNode<T>>>) {
        self.nodes.insert(None, node);
    }

    fn get_clone(&self, u: &Option<u8>) -> Option<sync::Arc<sync::RwLock<TrieNode<T>>>> {
        if let Some(n) = self.nodes.get(u) {
            Some(n.clone())
        } else {
            None
        }
    }

    fn is_exist(&self, u: &Option<u8>) -> bool {
        self.nodes.contains_key(u)
    }

    pub fn data_mut(&mut self) -> &mut Option<T> {
        &mut self.data
    }

    pub fn data_ref(&self) -> &Option<T> {
        &self.data
    }

    fn new(data: Option<T>) -> Self {
        Self {
            data: data,
            nodes: collections::HashMap::new()
        }
    }
}

pub struct TrieTree<T> {
    root: sync::Arc<sync::RwLock<TrieNode<T>>>
}

impl<T> TrieTree<T> {
    pub fn push(&mut self, data: &[u8], cf: fn() -> T) -> Result<sync::Arc<sync::RwLock<TrieNode<T>>>> {
        if data.is_empty() {
            return Err(Error::Simple(ErrorKind::RouteIsEmpty));
        }

        let mut node = self.root.clone();

        for item in data {
            let (n, is) = {
                match node.read().unwrap().get_clone(&Some(*item)) {
                    Some(n) => {
                        (n, true)
                    },
                    None => {
                        (node.read().unwrap().new_node(), false)
                    }
                }
            };
            if !is {
                node.write().unwrap().insert_node(*item, n.clone());
            }

            node = n;
        }

        let (leaf_node, is) = {
            match node.read().unwrap().get_clone(&None) {
                Some(n) => {
                    (n, true)
                },
                None => {
                    (node.read().unwrap().new_none(cf()), false)
                }
            }
        };
        if !is {
            node.write().unwrap().insert_none(leaf_node.clone());
        }

        Ok(leaf_node)
    }

    pub async fn find_from_stream(
        &self, stream: &mut Stream, length: &mut usize) -> Result<sync::Arc<sync::RwLock<TrieNode<T>>>> {

        *length = 0;

        let mut node = self.root.clone();

        loop {
            let item = match stream.lookup_next_one().await {
                Ok(v) => v,
                Err(err) => {
                    return Err(Error::Simple(ErrorKind::NotMatched));
                }
            };

            *length += 1;

            if item == b' ' {
                break;
            }

            let n = match node.read().unwrap().get_clone(&Some(item)) {
                Some(n) => {
                    n
                },
                None => {
                    return Err(Error::Content(
                            format!("UrlTrietree::find_from_stream get {} from node, not found"
                                    , item)));
                }
            };

            node = n;

            stream.skip_next_one();
        }

        let leaf_node = match node.read().unwrap().get_clone(&None) {
            Some(n) => {
                n
            },
            None => {
                return Err(Error::Simple(ErrorKind::NotMatched));
            }
        };

        Ok(leaf_node)
    }

    pub fn find(&self, data: &[u8], length: &mut usize) -> Result<sync::Arc<sync::RwLock<TrieNode<T>>>> {
        if data.is_empty() {
            return Err(Error::Simple(ErrorKind::RouteIsEmpty));
        }

        *length = 0;

        let mut node = self.root.clone();

        for item in data {
            *length += 1;

            if *item == b' ' {
                break;
            }

            let n = match node.read().unwrap().get_clone(&Some(*item)) {
                Some(n) => {
                    n
                },
                None => {
                    return Err(Error::Simple(ErrorKind::RouteIsEmpty));
                }
            };

            node = n;
        }

        let leaf_node = match node.read().unwrap().get_clone(&None) {
            Some(n) => {
                n
            },
            None => {
                return Err(Error::Simple(ErrorKind::NotMatched));
            }
        };

        Ok(leaf_node)
    }

    pub fn new() -> Self {
        Self {
            root: sync::Arc::new(sync::RwLock::new(TrieNode::new(None)))
        }
    }
}

