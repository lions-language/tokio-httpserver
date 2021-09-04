use std::sync;
use std::collections;

use crate::*;
use crate::stream;

pub struct TrieNode<T> {
    data: Option<T>,
    nodes: collections::HashMap<Option<u8>, sync::Arc<sync::RwLock<TrieNode<T>>>>
}

impl<T> TrieNode<T> {
    fn new_node(&self) -> sync::Arc<sync::RwLock<TrieNode<T>>> {
        sync::Arc::new(sync::RwLock::new(TrieNode::new(None)))
    }

    fn insert_node(&mut self, u: u8, node: sync::Arc<sync::RwLock<TrieNode<T>>>) {
        self.nodes.insert(Some(u.to_ascii_lowercase()), node);
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

pub struct Match<T> {
    node: sync::Arc<sync::RwLock<TrieNode<T>>>
}

impl<T> Match<T> {
    pub fn matched(&mut self, b: &u8) -> bool {
        let n = match self.node.read().unwrap().get_clone(&Some(b.to_ascii_lowercase())) {
            Some(n) => n,
            None => {
                return false;
            }
        };

        *&mut self.node = n;

        true
    }

    pub fn get_clone_unwrap(&self) -> sync::Arc<sync::RwLock<TrieNode<T>>> {
        self.node.read().unwrap().get_clone(&None).unwrap()
    }

    pub fn new(node: sync::Arc<sync::RwLock<TrieNode<T>>>) -> Self {
        Self {
            node: node
        }
    }
}

pub struct TrieTree<T> {
    root: sync::Arc<sync::RwLock<TrieNode<T>>>
}

impl<T> TrieTree<T> {
    pub fn push(&mut self, data: &[u8], t: T) -> Result<(sync::Arc<sync::RwLock<TrieNode<T>>>, bool)> {
        if data.is_empty() {
            return Err(Error::Simple(ErrorKind::RouteIsEmpty));
        }

        let mut node = self.root.clone();

        for item in data {
            let (n, is) = {
                match node.read().unwrap().get_clone(&Some(item.to_ascii_lowercase())) {
                    Some(n) => {
                        (n, true)
                    },
                    None => {
                        (node.read().unwrap().new_node(), false)
                    }
                }
            };
            if !is {
                node.write().unwrap().insert_node(item.to_ascii_lowercase(), n.clone());
            }

            node = n;
        }

        let (leaf_node, is) = {
            match node.read().unwrap().get_clone(&None) {
                Some(n) => {
                    (n, true)
                },
                None => {
                    (node.read().unwrap().new_none(t), false)
                }
            }
        };
        if !is {
            node.write().unwrap().insert_none(leaf_node.clone());
        }

        Ok((leaf_node, is))
    }

    pub async fn find_from_stream<S: stream::Stream>(
        &self, stream: &mut S, length: &mut usize) -> Result<sync::Arc<sync::RwLock<TrieNode<T>>>> {

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

            if item == b' ' || item == b':' {
                break;
            }

            let n = match node.read().unwrap().get_clone(&Some(item.to_ascii_lowercase())) {
                Some(n) => {
                    n
                },
                None => {
                    return Err(Error::Content(
                            format!("HeaderTrietree::find_from_stream get {} from node, not found"
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

            if item.to_ascii_lowercase() == b' ' {
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

    pub fn matched(&self) -> Match<T> {
        Match::<T>::new(self.root.clone())
    }

    pub fn new() -> Self {
        Self {
            root: sync::Arc::new(sync::RwLock::new(TrieNode::new(None)))
        }
    }
}

