use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};

pub trait PipeStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> PipeStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

pub struct Pipe {}
impl Pipe {
    pub async fn connect(pipe_name: &str) -> Result<Box<dyn PipeStream>> {
        #[cfg(target_os = "windows")]
        {
            use tokio::net::windows::named_pipe::ClientOptions;
            let client = ClientOptions::new().open(pipe_name)?;
            Ok(Box::new(client))
        }
        #[cfg(not(target_os = "windows"))]
        {
            use tokio::net::UnixStream;
            let stream = UnixStream::connect(pipe_name).await?;
            Ok(Box::new(stream))
        }
    }
}
