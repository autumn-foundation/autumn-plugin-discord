//! Health indicator tests. No network: a local TCP stub stands in for Discord.

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

fn stub_server(status: u16, body: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener binds");
    let addr = listener.local_addr().expect("listener has an addr");
    let body = body.to_owned();
    std::thread::spawn(move || {
        // Two connections: the tests call check_now() and then check().
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accepts a connection");
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn healthy_token_reports_up_with_application_name() {
    let base = stub_server(200, r#"{"id":"456","name":"TestBot"}"#);
    let client = DiscordClient::new(base, "good-token").expect("client builds");
    let health = DiscordHealth::new(client);

    assert_eq!(health.check_now().await.expect("check works"), "TestBot");

    let output = health.check().await;
    assert_eq!(output.status, HealthStatus::Up);
    assert_eq!(
        output.details.get("application"),
        Some(&serde_json::Value::String("TestBot".into()))
    );
}

#[tokio::test]
async fn bad_token_reports_down_with_detail() {
    let base = stub_server(401, r#"{"message":"401: Unauthorized"}"#);
    let client = DiscordClient::new(base, "bad-token").expect("client builds");
    let health = DiscordHealth::new(client);

    health
        .check_now()
        .await
        .expect_err("bad token fails the check");

    let output = health.check().await;
    assert_eq!(output.status, HealthStatus::Down);
    assert!(output.details.contains_key("error"));
}
