pub mod tcpstream;

enum Error {
    Content(String)
}

pub type Result<T> = std::result::Result<T, Error>;

