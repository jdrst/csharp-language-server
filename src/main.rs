use ::futures::future::try_join;
use anyhow::{Context, Result};
use home::home_dir;
use rust_search::SearchBuilder;
use serde_json::Value;
use std::process::Stdio;
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
    let mut log_dir = home_dir().unwrap().into_os_string();
    log_dir.push("/.roslyn/logs");

    let mut process = Command::new("Microsoft.CodeAnalysis.LanguageServer")
        .arg("--logLevel=Information")
        .arg("--extensionLogDirectory")
        .arg(log_dir)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let reader = BufReader::new(process.stdout.take().expect("Failed to capture stdout"));
    let roslyn_response = parse_roslyn_response(reader)
        .await
        .expect("Unable to parse response from server");

    let mut stream = UnixStream::connect(roslyn_response.pipe_name)
        .await
        .expect("Unable to connect to server stream");

    let (mut reader, mut writer) = stream.split();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let stream_to_stdout = io::copy(&mut reader, &mut stdout);
    let stdin_to_stream = async {
        let mut stdin = BufReader::new(stdin);
        loop {
            let mut buffer = vec![0; 3048];
            let bytes_read = stdin
                .read(&mut buffer)
                .await
                .expect("Unable to read incoming client notification");
            if bytes_read == 0 {
                break; // EOF reached
            }

            writer
                .write_all(&buffer[..bytes_read])
                .await
                .expect("Unable to forward client notification to server");

            let message = String::from_utf8(buffer[..bytes_read].to_vec())
                .expect("Unable to convert buffer to string");

            if message.contains("initialize") {
                let root_path = parse_root_path(&message)
                    .expect("Root path not part of initialize notification");
                let solution_to_open = find_solution_to_open(&root_path);

                if let Some(solution_to_open) = solution_to_open {
                    let open_solution_notification =
                        create_open_solution_notification(&solution_to_open);

                    writer
                        .write_all(open_solution_notification.as_bytes())
                        .await
                        .expect("Unable to send open solution notification to server");

                    break;
                }

                // TODO: Search for csproj files and send projects/open notification
                break;
            }
        }
        io::copy(&mut stdin, &mut writer).await
    };

    try_join(stdin_to_stream, stream_to_stdout).await.unwrap();
}

fn parse_root_path(notification: &str) -> Result<String> {
    let json_start = notification
        .find('{')
        .context("Notification was not json")?;

    let parsed_notification: Value = serde_json::from_str(&notification[json_start..])?;

    let root_path = parsed_notification["params"]["rootPath"]
        .as_str()
        .context("Root path")?;

    Ok(root_path.to_string())
}

async fn parse_roslyn_response(reader: BufReader<ChildStdout>) -> Result<RoslynResponse> {
    let first_line = reader
        .lines()
        .next_line()
        .await?
        .context("No lines to read")?;
    let parsed = serde_json::from_str::<RoslynResponse>(&first_line)?;
    Ok(parsed)
}

fn find_solution_to_open(root_path: &str) -> Option<String> {
    let solution_search: Vec<String> = SearchBuilder::default()
        .location(root_path)
        .ext("sln")
        .build()
        .collect();

    solution_search.first().map(|found| found.to_owned())
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

    create_notification(&message)
}

fn create_notification(body: &str) -> String {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let full_messsage = format!("{}{}", header, body);

    full_messsage
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
