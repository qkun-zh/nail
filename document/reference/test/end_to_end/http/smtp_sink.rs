
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct CapturedMail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

pub type MailBox = Arc<Mutex<Vec<CapturedMail>>>;

pub async fn start_sink() -> (u16, MailBox) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind smtp sink");
    let port = listener.local_addr().expect("sink addr").port();
    let inbox: MailBox = Arc::new(Mutex::new(Vec::new()));
    let inbox_task = inbox.clone();
    tokio::spawn(async move {
        loop {
            let Ok((socket, _peer)) = listener.accept().await else {
                break;
            };
            let inbox = inbox_task.clone();
            tokio::spawn(async move {
                let _ = handle_connection(socket, inbox).await;
            });
        }
    });
    (port, inbox)
}

async fn read_line(
    reader: &mut tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Option<String> {
    let mut line = String::new();
    match tokio::io::AsyncBufReadExt::read_line(reader, &mut line).await {
        Ok(0) => None,
        Ok(_) => {
            let trimmed = line.trim_end_matches(['\r', '\n']);
            Some(trimmed.to_string())
        }
        Err(_) => None,
    }
}

async fn handle_connection(socket: tokio::net::TcpStream, inbox: MailBox) -> std::io::Result<()> {
    let (read_half, mut write_half) = socket.into_split();
    let mut reader = tokio::io::BufReader::new(read_half);
    use tokio::io::AsyncWriteExt;

    write_half
        .write_all(b"220 nail-test-sink ESMTP\r\n")
        .await?;

    let mut _from = String::new();
    let mut to = String::new();
    let mut body_lines: Vec<String> = Vec::new();
    let mut in_data = false;

    while let Some(line) = read_line(&mut reader).await {
        let upper = line.to_ascii_uppercase();
        if in_data {
            if line == "." {
                let body = body_lines.join("\n");
                let subject = extract_subject(&body);
                inbox.lock().expect("sink inbox lock").push(CapturedMail {
                    to: to.clone(),
                    subject,
                    body,
                });
                body_lines.clear();
                to.clear();
                in_data = false;
                write_half.write_all(b"250 OK queued\r\n").await?;
                continue;
            }
            body_lines.push(line);
            continue;
        }
        if upper.starts_with("EHLO") || upper.starts_with("HELO") {
            write_half
                .write_all(b"250-nail-test-sink\r\n250-SIZE 10485760\r\n250 OK\r\n")
                .await?;
        } else if upper.starts_with("MAIL FROM:") {
            _from = line.trim_end().to_string();
            write_half.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("RCPT TO:") {
            to = extract_address(&line);
            write_half.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("DATA") {
            in_data = true;
            write_half
                .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")
                .await?;
        } else if upper.starts_with("QUIT") {
            write_half.write_all(b"221 Bye\r\n").await?;
            break;
        } else if upper.starts_with("AUTH") || upper.starts_with("STARTTLS") {
            write_half
                .write_all(b"235 2.7.0 Authentication successful\r\n")
                .await?;
        } else if upper.starts_with("RSET") {
            write_half.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("NOOP") {
            write_half.write_all(b"250 OK\r\n").await?;
        } else {
            write_half.write_all(b"250 OK\r\n").await?;
        }
    }
    Ok(())
}

fn extract_address(line: &str) -> String {
    if let Some(start) = line.find('<') {
        let rest = &line[start + 1..];
        if let Some(end) = rest.find('>') {
            return rest[..end].to_string();
        }
    }
    line.to_string()
}

fn extract_subject(body: &str) -> String {
    body.lines()
        .find(|l| l.to_ascii_lowercase().starts_with("subject:"))
        .map(|l| {
            l.split_once(':')
                .map(|(_, rest)| rest.trim().to_string())
                .unwrap_or_default()
        })
        .unwrap_or_default()
}
