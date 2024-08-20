use ::futures::future::try_join;
use anyhow::{Context, Result};
use home::home_dir;
use rust_search::SearchBuilder;
use serde_json::{json, Value};
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
    let mut log_dir = home_dir()
        .expect("Unable to find home directory")
        .into_os_string();
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

    let (reader, mut writer) = stream.split();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let stream_to_stdout = async {
        let mut reader = BufReader::new(reader);
        loop {
            let mut buffer = vec![0; 3048];
            let bytes_read = reader
                .read(&mut buffer)
                .await
                .expect("Unable to read incoming server notification");

            if bytes_read == 0 {
                break; // EOF reached
            }

            let notification = String::from_utf8(buffer[..bytes_read].to_vec())
                .expect("Unable to convert buffer to string");

            if notification.contains("capabilities") {
                let patched_result_notification = force_pull_diagnostics_hack(&notification)?;

                stdout
                    .write_all(patched_result_notification.as_bytes())
                    .await?;
            }

            if notification.contains("workspace/projectInitializationComplete") {
                let refresh_notification = NotificationArrayParams {
                    jsonrpc: "2.0".to_string(),
                    method: "workspace/diagnostic/refresh".to_string(),
                    params: vec![],
                };

                stdout
                    .write_all(refresh_notification.serialize().as_bytes())
                    .await?;
                stdout.write_all(notification.as_bytes()).await?;

                break;
            }

            stdout
                .write_all(&buffer[..bytes_read])
                .await
                .expect("Unable to forward client notification to server");
        }

        io::copy(&mut reader, &mut stdout).await
    };

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

            let notification = String::from_utf8(buffer[..bytes_read].to_vec())
                .expect("Unable to convert buffer to string");

            if notification.contains("initialize") {
                let root_path = parse_root_path(&notification)
                    .expect("Root path not part of initialize notification");

                let solution_files = find_extension(&root_path, "sln");
                let solution_to_open = solution_files.first().map(|found| found.to_owned());

                if let Some(solution_to_open) = solution_to_open {
                    let open_solution_notification =
                        create_open_solution_notification(&solution_to_open);

                    writer
                        .write_all(open_solution_notification.as_bytes())
                        .await
                        .expect("Unable to send open solution notification to server");

                    break;
                }

                let project_files = find_extension(&root_path, "csproj");
                let open_projects_notification = create_open_projects_notification(project_files);

                writer
                    .write_all(open_projects_notification.as_bytes())
                    .await
                    .expect("Unable to send open projects notification to server");

                break;
            }
        }
        io::copy(&mut stdin, &mut writer).await
    };

    try_join(stdin_to_stream, stream_to_stdout)
        .await
        .expect("Will never finish");
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

fn find_extension(root_path: &str, extension: &str) -> Vec<String> {
    SearchBuilder::default()
        .location(root_path)
        .ext(extension)
        .build()
        .collect()
}

fn create_open_solution_notification(file_path: &str) -> String {
    let notification = Notification {
        jsonrpc: "2.0".to_string(),
        method: "solution/open".to_string(),
        params: Params::Solution(SolutionParams {
            solution: path_to_uri(file_path),
        }),
    };

    notification.serialize()
}

fn path_to_uri(file_path: &str) -> String {
    format!("file://{file_path}")
}

fn create_open_projects_notification(file_paths: Vec<String>) -> String {
    let uris: Vec<String> = file_paths
        .iter()
        .map(|file_path| path_to_uri(file_path))
        .collect();

    let notification = Notification {
        jsonrpc: "2.0".to_string(),
        method: "project/open".to_string(),
        params: Params::Project(ProjectParams { projects: uris }),
    };

    notification.serialize()
}

fn force_pull_diagnostics_hack(notification: &str) -> Result<String, std::io::Error> {
    let json_start = notification.find('{').ok_or(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "No JSON start found",
    ))?;
    let mut parsed_notification: Value = serde_json::from_str(&notification[json_start..])?;

    let diagnostic_provider = json!({
        "interFileDependencies": true,
        "workDoneProgress": true,
        "workspaceDiagnostics": true
    });

    parsed_notification["result"]["capabilities"]["diagnosticProvider"] = diagnostic_provider;

    Ok(create_notification(&parsed_notification.to_string()))
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
enum Params {
    Solution(SolutionParams),
    Project(ProjectParams),
}

#[derive(Serialize, Debug)]
struct Notification {
    jsonrpc: String,
    method: String,
    params: Params,
}

#[derive(Serialize, Debug)]
struct NotificationArrayParams {
    jsonrpc: String,
    method: String,
    params: Vec<String>,
}

#[derive(Serialize, Debug)]
struct SolutionParams {
    solution: String,
}

#[derive(Serialize, Debug)]
struct ProjectParams {
    projects: Vec<String>,
}

impl Notification {
    fn serialize(self) -> String {
        let body = serde_json::to_string(&self).expect("Unable to serialize notification");
        create_notification(&body)
    }
}

impl NotificationArrayParams {
    fn serialize(self) -> String {
        let body = serde_json::to_string(&self).expect("Unable to serialize notification");
        create_notification(&body)
    }
}

fn create_notification(body: &str) -> String {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let full_messsage = format!("{}{}", header, body);

    full_messsage
}
