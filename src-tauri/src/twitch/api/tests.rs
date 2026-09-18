use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

// A tiny local HTTP fixture verifies real reqwest form/header behavior and
// Twitch's non-standard {status,message} errors without a live registration.
async fn server(status: u16, body: &str) -> (HttpTwitchApi, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        loop {
            let mut buffer = [0; 1024];
            let count = socket.read(&mut buffer).await.unwrap();
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            let text = String::from_utf8_lossy(&request);
            if let Some((headers, body)) = text.split_once("\r\n\r\n") {
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if body.len() >= length {
                    break;
                }
            }
        }
        socket.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8(request).unwrap()
    });
    let mut api = HttpTwitchApi::new().unwrap();
    api.base = base;
    (api, handle)
}

fn form(request: &str) -> std::collections::HashMap<String, String> {
    url::form_urlencoded::parse(request.split_once("\r\n\r\n").unwrap().1.as_bytes())
        .into_owned()
        .collect()
}

#[tokio::test]
async fn actual_device_and_refresh_forms_never_include_client_secret() {
    let (api, request) = server(200, r#"{"device_code":"private","user_code":"PUBLIC","verification_uri":"https://www.twitch.tv/activate","expires_in":1800,"interval":5}"#).await;
    api.begin("client").await.unwrap();
    let request = form(&request.await.unwrap());
    assert_eq!(request["client_id"], "client");
    assert_eq!(request["scopes"], "");
    assert!(!request.contains_key("client_secret"));

    let (api, request) = server(
        200,
        r#"{"access_token":"a","refresh_token":"r","token_type":"bearer"}"#,
    )
    .await;
    api.refresh("client", "refresh+&value").await.unwrap();
    let request = form(&request.await.unwrap());
    assert_eq!(request["refresh_token"], "refresh+&value");
    assert_eq!(request["grant_type"], "refresh_token");
    assert!(!request.contains_key("client_secret"));
}

#[tokio::test]
async fn twitch_pending_response_and_rfc_grant_are_supported() {
    let (api, request) = server(400, r#"{"status":400,"message":"authorization_pending"}"#).await;
    assert!(matches!(
        api.poll("client", "private+code").await.unwrap(),
        PollResult::Pending
    ));
    let request = form(&request.await.unwrap());
    assert_eq!(request["grant_type"], DEVICE_GRANT);
    assert_eq!(request["device_code"], "private+code");
    assert!(request.contains_key("scopes"));
    assert!(!request.contains_key("client_secret"));
}

#[tokio::test]
async fn validate_header_is_rust_owned_and_unauthorized_is_typed() {
    let (api, request) = server(401, r#"{"message":"invalid access token"}"#).await;
    assert_eq!(
        api.validate("private-token").await.unwrap_err().code,
        ErrorCode::AuthInvalid
    );
    assert!(
        request
            .await
            .unwrap()
            .to_lowercase()
            .contains("authorization: oauth private-token")
    );
}

#[tokio::test]
async fn untrusted_provider_error_bodies_do_not_escape() {
    let (api, request) = server(400, r#"{"message":"private-secret-should-not-escape"}"#).await;
    let error = match api.poll("client", "code").await {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains("private-secret")
    );
    request.await.unwrap();
}
