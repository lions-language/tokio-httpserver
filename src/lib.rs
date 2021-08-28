pub mod tcpstream;

pub enum Error {
    Content(String)
}

pub type Result<T> = std::result::Result<T, Error>;

pub type ByteArray = Vec<u8>;

