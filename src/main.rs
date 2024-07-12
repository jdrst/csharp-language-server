use ::futures::future::try_join;
use anyhow::{Context, Result};
use rust_search::SearchBuilder;
use serde_json::json;
use std::{
    env::current_dir,
    io::{BufRead, BufReader},
    process::{ChildStdout, Command, Stdio},
};
use tokio::{
    io::{self, AsyncWriteExt},
    net::UnixStream,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct RoslynResponse {
    #[serde(rename = "pipeName")]
    pipe_name: String,
}

#[tokio::main]
async fn main() {
    // Start roslyn language server process
    let mut process = Command::new("Microsoft.CodeAnalysis.LanguageServer")
        .arg("--logLevel=Information")
        .arg("--extensionLogDirectory")
        .arg("~/roslyn-language-server/logs")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    // Get named pipe
    let reader = BufReader::new(process.stdout.take().expect("Failed to capture stdout"));
    let parsed_roslyn_response =
        parse_first_line(reader).expect("Unable to parse response from Roslyn");

    println!("Named pipe: {}", parsed_roslyn_response.pipe_name);

    // find .sln file
    let execution_path = current_dir().unwrap();
    let solution_search: Vec<String> = SearchBuilder::default()
        .location(execution_path)
        .ext("sln")
        .build()
        .collect();

    let found_solution = solution_search
        .first()
        .expect("Unable to find solution file");

    println!("Found solution: {}", found_solution);

    // Send open solution lsp command
    let open_solution_notification = create_open_solution_notification(found_solution);
    let mut stream = UnixStream::connect(parsed_roslyn_response.pipe_name)
        .await
        .unwrap();

    let message = json!(open_solution_notification).to_string();
    stream.write_all(message.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();

    forward_in_out(stream).await.unwrap();
}

fn parse_first_line(reader: BufReader<ChildStdout>) -> Result<RoslynResponse> {
    let first_line = reader.lines().next().context("Unable to read line")??;
    let parsed = serde_json::from_str::<RoslynResponse>(&first_line)?;
    Ok(parsed)
}

fn create_open_solution_notification(file_path: &str) -> String {
    let notificatin = Notification {
        jsonrpc: "2.0".to_string(),
        method: "solution/open".to_string(),
        params: SolutionParams{ solution: FileUri { uri: format!("file://{file_path}")} } 
    };

    let message = serde_json::to_string(&notificatin).expect("Unable to serialize notification");

    let header = format!("Content-Length: {}\r\n\r\n", message.len());
    format!("{}{}", header, message)
}

async fn forward_in_out(mut socket: UnixStream) -> Result<()> {
    let (mut reader, mut writer) = socket.split();

    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    let stdin_to_socket = io::copy(&mut stdin, &mut writer);
    let socket_to_stdout = io::copy(&mut reader, &mut stdout);

    try_join(stdin_to_socket, socket_to_stdout).await.unwrap();

    Ok(())
}

#[derive(Serialize, Debug)]
struct Notification {
    jsonrpc: String,
    method: String,
    params: SolutionParams,
}

#[derive(Serialize, Debug)]
struct SolutionParams {
    solution: FileUri,
}

#[derive(Serialize, Debug)]
struct FileUri {
    uri: String,
}
