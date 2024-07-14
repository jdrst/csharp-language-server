use ::futures::future::try_join;
use anyhow::{Context, Result};
use rust_search::SearchBuilder;
use std::{
    borrow::BorrowMut,
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
    let roslyn_response = start_roslyn_server();

    let solution_to_open = find_solution_to_open();
    let open_solution_notification = create_open_solution_notification(&solution_to_open);

    let mut stream = open_stream(&roslyn_response.pipe_name).await;
    send_open_solution_notification(&open_solution_notification, stream.borrow_mut()).await;

    forward_in_out_to_socket(stream).await.unwrap();
}

fn start_roslyn_server() -> RoslynResponse {
    let mut process = Command::new("Microsoft.CodeAnalysis.LanguageServer")
        .arg("--logLevel=Information")
        .arg("--extensionLogDirectory")
        .arg("~/roslyn-language-server/logs")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let reader = BufReader::new(process.stdout.take().expect("Failed to capture stdout"));
    let parsed_roslyn_response =
        parse_first_line(reader).expect("Unable to parse response from Roslyn");

    println!("Socket name: {}", parsed_roslyn_response.pipe_name);

    parsed_roslyn_response
}

fn find_solution_to_open() -> String {
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

    found_solution.to_owned()
}

async fn open_stream(socket_name: &str) -> UnixStream {
    UnixStream::connect(socket_name)
        .await
        .expect("Unable to connect to server stream")
}

async fn send_open_solution_notification(notification: &str, stream: &mut UnixStream) {
    stream
        .write_all(notification.as_bytes())
        .await
        .expect("Unable to send open notification to stream");
    stream.flush().await.expect("Unable to flush stream");
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
        params: SolutionParams {
            solution: format!("file://{file_path}"),
        },
    };

    let message = serde_json::to_string(&notificatin).expect("Unable to serialize notification");

    let header = format!("Content-Length: {}\r\n\r\n", message.len());
    let full_messsage = format!("{}{}", header, message);
    println!("{full_messsage}");

    full_messsage
}

async fn forward_in_out_to_socket(mut socket: UnixStream) -> Result<()> {
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
    solution: String,
}
