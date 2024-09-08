use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufReader},
    process::ChildStdout,
};

#[derive(Serialize, Deserialize)]
pub struct RoslynResponse {
    #[serde(rename = "pipeName")]
    pub pipe_name: String,
}

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

pub async fn parse_roslyn_response(reader: BufReader<ChildStdout>) -> Result<RoslynResponse> {
    let first_line = reader
        .lines()
        .next_line()
        .await?
        .context("No lines to read")?;
    let parsed = serde_json::from_str::<RoslynResponse>(&first_line)?;
    Ok(parsed)
}
