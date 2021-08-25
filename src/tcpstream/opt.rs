use tokio::net;
use tokio::io::{AsyncReadExt};
use tokio::net::tcp::OwnedReadHalf;

use std::io;
use std::collections;

use crate::*;
use crate::stream;
use crate::stream::Stream as StreamTrait;

const BUF_SIZE: usize = 4096;

pub struct Backtrace<'a> {
    stream: &'a mut Stream,

    index: usize
}

#[async_trait::async_trait]
impl<'a> stream::Backtrace for Backtrace<'a> {
    fn commit(&mut self) {
        self.stream.skip_next_n(self.index);
    }

    async fn lookup_next_one(&mut self) -> Result<u8> {
        let index = self.index;
        if index >= self.stream.buffer.len() {
            if let Err(err) = self.stream.read_from_stream().await {
                return Err(err);
            };
        }

        match self.stream.buffer.get(index) {
            Some(v) => {
                Ok(*v)
            },
            None => {
                Err(Error::Content(
                        format!("TcpStream::Backtrace::lookup_next_one get from buffer is none")))
            }
        }
    }

    async fn take_next_one(&mut self) -> Result<u8> {
        let index = self.index + 1;
        if index >= self.stream.buffer.len() {
            if let Err(err) = self.stream.read_from_stream().await {
                return Err(err);
            };
        }

        match self.stream.buffer.get(index) {
            Some(v) => {
                self.index += 1;
                Ok(*v)
            },
            None => {
                Err(Error::Content(
                        format!("TcpStream::Backtrace::take_next_one get from buffer is none")))
            }
        }
    }

    fn skip_next_n(&mut self, n: usize) {
        self.index += n;
    }

    fn skip_next_one(&mut self) {
        self.skip_next_n(1);
    }

    fn rollback(&self) -> usize {
        self.index
    }

    fn take_all_iter(&mut self) -> std::collections::vec_deque::Drain<'_, u8> {
        self.stream.buffer.drain(0..self.index)
    }

    fn take_all_vec(&mut self) -> Vec<u8> {
        self.stream.buffer.drain(0..self.index).collect()
    }
}

impl<'a> Backtrace<'a> {
    fn new(stream: &'a mut Stream) -> Self {
        Self {
            stream: stream,
            index: 0
        }
    }
}

/// ////////////////////////////////
pub struct Stream {
    buffer: collections::VecDeque<u8>,

    count: usize,

    stream: OwnedReadHalf,
}

#[async_trait::async_trait]
impl stream::Stream for Stream {
    type Backtrace<'a> = Backtrace<'a>;

    async fn lookup_next_one(&mut self) -> Result<u8> {
        // println!("before lookup_next_one");

        if self.buffer.len() == 0 {
            if let Err(err) = self.read_from_stream().await {
                return Err(err);
            };
        }

        // println!("after lookup_next_one");

        match self.buffer.front() {
            Some(c) => {
                Ok(*c)
            },
            None => {
                Err(Error::Content(
                        format!("TcpStream::lookup_next_one get front from buffer is none")))
            }
        }
    }

    async fn take_next_one(&mut self) -> Result<u8> {
        if self.buffer.len() == 0 {
            if let Err(err) = self.read_from_stream().await {
                return Err(err);
            };
        }

        match self.buffer.pop_front() {
            Some(c) => {
                Ok(c)
            },
            None => {
                Err(Error::Content(
                        format!("TcpStream::take_next_one pop front from buffer is none")))
            }
        }
    }

    async fn skip_white_space(&mut self) {
        loop {
            let n = match self.lookup_next_one().await {
                Ok(n) => n,
                Err(_) => {
                    break;
                }
            };

            if n == b' ' {
                self.skip_next_one();
            } else {
                break;
            }
        }
    }

    fn skip_next_one(&mut self) -> Result<()> {
        self.skip_next_n(1)
    }

    fn skip_next_n(&mut self, n: usize) -> Result<()> {
        if n > self.buffer.len() {
            return Err(Error::Content(
                    format!("TcpStream::skip_next_n n[{}] > self.buffer.len() [{}]"
                            , n, self.buffer.len())));
        }
        
        self.buffer.drain(0..n);

        self.count += n;

        Ok(())
    }

    fn take_n_iter(&mut self, n: usize) -> Result<std::collections::vec_deque::Drain<'_, u8>> {
        if n > self.buffer.len() {
            return Err(Error::Content(
                    format!("TcpStream::take_n_iter n[{}] > self.buffer.len() [{}]"
                            , n, self.buffer.len())));
        }

        Ok(self.buffer.drain(0..n))
    }

    fn take_n_vec(&mut self, n: usize) -> Result<Vec<u8>> {
        if n > self.buffer.len() {
            return Err(Error::Content(
                    format!("TcpStream::take_n_vec n[{}] > self.buffer.len() [{}]"
                            , n, self.buffer.len())));
        }

        Ok(self.buffer.drain(0..n).collect())
    }

    fn reset_count(&mut self) {
        self.count = 0;
    }

    fn count_clone(&self) -> usize {
        self.count
    }

    fn count_ref(&self) -> &usize {
        &self.count
    }

    fn create_backtrace<'a>(&'a mut self) -> Self::Backtrace<'a> {
        Backtrace::new(self)
    }

    async fn read(&mut self, size: usize) -> Result<ByteArray> {
        let mut buf = Vec::with_capacity(size);

        let mut remain_len: usize = size;

        loop {
            // if let Err(err) = self.stream.readable().await {
            //     return Err(Error::Io(err));
            // };
            // let n = match self.stream.try_read(&mut buffer) {

            let mut buffer = [0; BUF_SIZE];

            let n = match self.stream.read(&mut buffer).await {
                Ok(0) => {
                    return Err(Error::Simple(ErrorKind::SocketClose));
                },
                Ok(n) => {
                    n
                },
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                },
                Err(e) => {
                    return Err(Error::Io(e));
                }
            };

            if n < remain_len {
                remain_len -= n;
                buf.extend(&buffer[0..n]);
                continue;
            }

            if n == remain_len {
                buf.extend(&buffer[0..n]);
                break;
            }

            if n > remain_len {
                buf.extend(&buffer[0..remain_len]);
                self.buffer.extend(&buffer[remain_len..n]);
                break;
            }
        }

        return Ok(buf);
    }
}

impl Stream {
    async fn read_from_stream(&mut self) -> Result<()> {
        loop {
            // if let Err(err) = self.stream.readable().await {
            //     return Err(Error::Io(err));
            // };
            // let n = match self.stream.try_read(&mut buf) {

            let mut buf = [0; BUF_SIZE];

            // println!("before read_from_stream");

            let n = match self.stream.read(&mut buf).await {
                Ok(0) => {
                    // println!("read_from_stream Ok(0)");
                    return Err(Error::Simple(ErrorKind::SocketClose));
                },
                Ok(n) => {
                    // println!("read_from_stream Ok({})", n);
                    n
                },
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // println!("read_from_stream Err(ref e)");
                    continue;
                },
                Err(e) => {
                    // println!("read_from_stream Err(e)");
                    return Err(Error::Io(e));
                }
            };

            self.buffer.extend(&buf[0..n]);

            break;
        }

        Ok(())
    }

    pub fn new(stream: OwnedReadHalf) -> Self {
        Self {
            buffer: collections::VecDeque::new(),

            count: 0 as usize,

            stream: stream
        }
    }
}

