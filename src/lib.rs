use std::env::current_dir;

use rust_search::SearchBuilder;
use serde::Serialize;
use tokio::net::UnixStream;

pub async fn open_stream(socket_name: &str) -> UnixStream {
    UnixStream::connect(socket_name)
        .await
        .expect("Unable to connect to server stream")
}

pub fn find_solution_to_open() -> String {
    let execution_path = current_dir().unwrap();
    let solution_search: Vec<String> = SearchBuilder::default()
        .location(execution_path)
        .ext("sln")
        .build()
        .collect();

    let found_solution = solution_search
        .first()
        .expect("Unable to find solution file");

    found_solution.to_owned()
}

pub fn create_open_solution_notification(file_path: &str) -> String {
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

pub fn create_notification(body: &str) -> String {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let full_messsage = format!("{}{}", header, body);

    full_messsage
}

#[derive(Serialize, Debug)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: SolutionParams,
}

#[derive(Serialize, Debug)]
pub struct SolutionParams {
    pub solution: String,
}
