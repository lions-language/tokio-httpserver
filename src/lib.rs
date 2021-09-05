pub mod stream_handler;
// pub mod route;

pub type Any = Box<dyn std::any::Any>;
pub type SendAny = Box<dyn std::any::Any + Send + Sync>;

type AnyResult = std::result::Result<Any, String>;
type DescResult = std::result::Result<(), String>;

pub enum Error {
    Content(String)
}

pub type Result<T> = std::result::Result<T, Error>;

pub type Byte = u8;
pub type ByteSlice = [u8];
pub type ByteArray = Vec<u8>;

pub fn new_none_bytearray() -> ByteArray {
    Vec::with_capacity(0)
}

pub fn new_bytearray() -> ByteArray {
    Vec::new()
}

////////////////////////////
#[derive(Clone)]
pub struct RegisterOptions {
    pub header_value_type: HeaderValueType,
    pub body_type: BodyType
}

impl Default for RegisterOptions {
    fn default() -> Self {
        Self {
            header_value_type: HeaderValueType::Utf8String,
            body_type: BodyType::Utf8String
        }
    }
}

////////////////////////////
pub struct Context {
    pub data: SendAny
}

pub type SharedContext = Arc<RwLock<Context>>;

/////////////////////////////
pub enum HeaderValueType {
    Integer,
    ByteArray,
    Utf8String,
    Custom
}

/////////////////////////////
pub enum BodyType {
    ByteArray,
    Utf8String,
    Custom
}

/////////////////////////////////////////
#[derive(Eq, PartialEq, Debug, Hash)]
pub enum Method {
    Post,
    Put,
    Get,
    Delete,
    Unknown
}

impl Default for Method {
    fn default() -> Self {
        Method::Unknown
    }
}

#[derive(Debug, Default)]
pub struct Version(ByteArray);

impl Version {
    fn push(&mut self, v: Byte) {
        self.0.push(v);
    }

    pub fn version(&self) -> &[Byte] {
        &self.0
    }

    pub fn new() -> Self {
        Self(new_bytearray())
    }
}

//////////////////////////////
pub type HeaderIndex = usize;

pub enum HeaderValue {
    Integer(u64),
    ByteArray(ByteArray),
    Utf8String(String),
    Custom(SendAny)
}

#[derive(Debug, Default)]
pub struct Headers {
    indexs: Vec<HeaderValue>,
    next_index: HeaderIndex,
    bytearrays: std::collections::HashMap<ByteArray, HeaderValue>
}

impl Headers {
    fn insert_bytearray(&mut self, key: ByteArray, value: ByteArray) -> Result<()> {
        if let Some(_) = self.bytearrays.get(&key) {
            return Err(Error::Content(format!("Headers::insert_bytearray, [key={:?}] not found", key)));
        };

        self.bytearrays.insert(key, value);

        Ok(())
    }

    fn push_index(&mut self, key: HeaderIndex, value: SendAny) -> Result<()> {
        if key != self.next_index {
            return Err(Error::Content(format!("Headers::insert_bytearray, [key-index={}]", key)));
        }

        self.indexs.push(value);

        self.next_index += 1;

        Ok(())
    }

    fn get_index(&self, key: &HeaderIndex) -> Option<&SendAny> {
        if self.indexs.len() == 0 {
            return None;
        }

        if *key > self.indexs.len() - 1 {
            return None;
        }

        Some(&self.indexs[*key])
    }

    fn new() -> Self {
        Self {
            indexs: Vec::new(),
            next_index: 0,
            bytearrays: std::collections::HashMap::new()
        }
    }
}

/////////////////////////////
pub enum Body {
    ByteArray(Vec<u8>),
    Utf8String(String),
    Custom(SendAny)
}

#[derive(Default)]
pub struct RequestHeader {
    pub method: Method,
    pub version: Version,
    pub headers: Box<Headers>
}

pub struct Request {
    pub header: Box<RequestHeader>,
    pub body: Option<Body>
}

pub type SharedResponseSender = std::sync::Arc<tokio::sync::RwLock<tokio::sync::mpsc::Sender<ResponseContent>>>;

pub struct ResponseCaptial {
    version: ByteArray,
    status: u16,
    status_desc: ByteArray
}

impl Default for ResponseCaptial {
    fn default() -> Self {
        Self {
            version: "HTTP/1.1".as_bytes().to_vec(),
            status: 200,
            status_desc: "OK".as_bytes().to_vec()
        }
    }
}

#[derive(Default)]
pub struct ResponseUncheckHeaders {
    bytes: ByteArray
}

#[derive(Default)]
pub struct ResponseContent {
    pub captial: ResponseCaptial,
    pub headers: ResponseUncheckHeaders,
    pub body: Body
}

pub struct Response<Writer: tokio::io::AsyncWrite + Send> {
    writer: std::sync::Arc<tokio::sync::RwLock<Writer>>,
    content: ResponseContent
}

