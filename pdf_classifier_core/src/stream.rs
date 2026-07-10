use std::env::var;
use std::io::Write;
use std::net::TcpStream;

use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;

#[derive(thiserror::Error, Debug)]
pub enum StreamerError {
    #[error(
        "The \"CLASSIFIER_OUTPUT_PORT\" environment variable wasn't defined, ensure startup via the Python front-end."
    )]
    MissingOutputPortEnvVar,

    #[error("Couldn't cast port string to a u16!")]
    FailedToCastPort,

    #[error("Couldn't connect to stream: {0}")]
    FailedToConnectToStream(String),

    #[error("Failed to length to stream: {0}")]
    FailedToWriteLength(String),

    #[error("Failed to write data to stream: {0}")]
    FailedToWriteData(String),

    #[error("Failed to flush stream after writing data: {0}")]
    FailedToFlushStream(String),
}

/// Streams information back to the Python front-end.
#[derive(Debug)]
pub struct Streamer {
    stream: TcpStream,
}

type StreamerResult<T> = std::result::Result<T, StreamerError>;

impl Streamer {
    pub fn new() -> StreamerResult<Self> {
        let port: u16 = var("CLASSIFIER_OUTPUT_PORT")
            .map_err(|_| StreamerError::MissingOutputPortEnvVar)?
            .parse()
            .map_err(|_| StreamerError::FailedToCastPort)?;

        let stream = TcpStream::connect(("127.0.0.1", port))
            .map_err(|e| StreamerError::FailedToConnectToStream(e.to_string()))?;

        Ok(Self { stream })
    }

    pub fn send_data(
        &mut self,
        page: Page,
        class: KnownObject,
        payload: &[u8],
    ) -> StreamerResult<()> {
        self.stream
            .write_all(&page.0.to_le_bytes())
            .map_err(|e| StreamerError::FailedToWriteData(e.to_string()))?;

        let class_str = class.to_string();
        let class_to_bytes = class_str.as_bytes();
        let class_len = &(class_to_bytes.len() as u32).to_le_bytes();
        self.stream
            .write_all(class_len)
            .map_err(|e| StreamerError::FailedToWriteLength(e.to_string()))?;

        self.stream
            .write_all(class_to_bytes)
            .map_err(|e| StreamerError::FailedToWriteData(e.to_string()))?;

        let payload_len = (payload.len() as u32).to_le_bytes();
        self.stream
            .write_all(&payload_len)
            .map_err(|e| StreamerError::FailedToWriteLength(e.to_string()))?;

        self.stream
            .write_all(payload)
            .map_err(|e| StreamerError::FailedToWriteData(e.to_string()))?;

        self.stream
            .flush()
            .map_err(|e| StreamerError::FailedToFlushStream(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::generated::generated_object_types::KnownObject;
    use crate::page::Page;
    use crate::stream::Streamer;
    use std::io::Read;
    use std::net::TcpListener;

    const TEST_DATA: &'static [u8] = b"test_data";

    #[test]
    fn test_spawn_stream_and_write() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        unsafe { std::env::set_var("CLASSIFIER_OUTPUT_PORT", port.to_string()) };

        std::thread::spawn(move || {
            let mut streamer = Streamer::new().expect("should've created stramer");
            streamer
                .send_data(Page(0), KnownObject::UNKNOWN, TEST_DATA)
                .expect("should've sent data");
        });

        let (mut conn, _) = listener.accept().unwrap();

        let mut page_num = [0u8; 4];
        conn.read_exact(&mut page_num)
            .expect("should've read page num");
        let _page_num = u32::from_le_bytes(page_num);

        let mut class_size = [0u8; 4];
        conn.read_exact(&mut class_size)
            .expect("should've read class size");
        let size = u32::from_le_bytes(class_size) as usize;

        let mut class = vec![0u8; size];
        conn.read_exact(&mut class)
            .expect("should've read class name");
        assert_eq!(
            class.as_slice(),
            KnownObject::UNKNOWN.to_string().as_bytes()
        );

        let mut data_len_buf = [0u8; 4];
        conn.read_exact(&mut data_len_buf)
            .expect("should've read data length");
        let data_len = u32::from_le_bytes(data_len_buf) as usize;

        let mut data = vec![0u8; data_len];
        conn.read_exact(&mut data).expect("should've read data");

        assert_eq!(data, TEST_DATA)
    }
}
