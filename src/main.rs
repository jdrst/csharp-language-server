use std::process::Stdio;

use ::futures::future::try_join;
use anyhow::{Context, Result};
use roslyn_language_server::{
    create_open_solution_notification, find_solution_to_open, open_stream,
};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    process::{ChildStdout, Command},
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct RoslynResponse {
    #[serde(rename = "pipeName")]
    pipe_name: String,
}

#[tokio::main]
async fn main() {
    let roslyn_response = start_roslyn_server().await;
    let stream = open_stream(&roslyn_response.pipe_name).await;

    forward_in_out_to_socket(stream).await.unwrap();
}

async fn start_roslyn_server() -> RoslynResponse {
    let mut process = Command::new("Microsoft.CodeAnalysis.LanguageServer")
        .arg("--logLevel=Information")
        .arg("--extensionLogDirectory")
        .arg("~/roslyn-language-server/logs")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let reader = BufReader::new(process.stdout.take().expect("Failed to capture stdout"));
    let parsed_roslyn_response = parse_first_line(reader)
        .await
        .expect("Unable to parse response from Roslyn");

    println!("Socket name: {}", parsed_roslyn_response.pipe_name);

    parsed_roslyn_response
}

async fn parse_first_line(reader: BufReader<ChildStdout>) -> Result<RoslynResponse> {
    let first_line = reader
        .lines()
        .next_line()
        .await?
        .context("No lines to read")?;
    let parsed = serde_json::from_str::<RoslynResponse>(&first_line)?;
    Ok(parsed)
}

async fn forward_in_out_to_socket(mut socket: UnixStream) -> Result<()> {
    let (mut reader, mut writer) = socket.split();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let socket_to_stdout = io::copy(&mut reader, &mut stdout);
    let stdin_to_socket = async {
        let mut stdin = BufReader::new(stdin);
        loop {
            let mut buffer = vec![0; 2048];
            let bytes_read = stdin.read(&mut buffer).await?;
            if bytes_read == 0 {
                break; // EOF reached
            }

            writer.write_all(&buffer[..bytes_read]).await?;

            let message = String::from_utf8(buffer[..bytes_read].to_vec()).unwrap();
            if message.contains("initialize") {
                let solution_to_open = find_solution_to_open();
                let open_solution_notification =
                    create_open_solution_notification(&solution_to_open);
                writer
                    .write_all(open_solution_notification.as_bytes())
                    .await?;
            }
        }
        Ok(())
    };

    try_join(stdin_to_socket, socket_to_stdout).await.unwrap();

    Ok(())
}
