use std::io;
use std::sync::Arc;
use rustls::ServerConfig;
use crate::runtime::net::{AsyncConnectionFuture, AsyncTcpStream};
use crate::runtime::AsyncProtocol;


#[derive(Debug)]
pub struct SmtpSession {
    pub client_domain: String,
    pub from: String,
    pub to: Vec<String>,
    pub cc: String,
    pub date: String,
    pub subject: String,
    pub content: String,
    pub is_data: bool,
    pub use_tls: bool,
    pub quit: bool,
}

impl SmtpSession {
    pub fn new() -> Self {
        Self {
            client_domain: String::new(),
            from: String::new(),
            to: Vec::new(),
            cc: String::new(),
            date: String::new(),
            subject: String::new(),
            content: String::new(),
            is_data: false,
            use_tls: false,
            quit: false,
        }
    }
}

pub struct Smtp {
    domain: String,
    config: Option<Arc<ServerConfig>>,
    post_handle: Box<dyn Fn(SmtpSession) -> AsyncConnectionFuture<'static, io::Result<usize>> + Send + Sync>,
}

impl AsyncProtocol for Smtp {

    fn handle_async_connection(&self, mut stream:AsyncTcpStream)
    -> AsyncConnectionFuture<'_, io::Result<usize>>
    { Box::pin( async move {
        stream.write_all( format!("220 {} ESMTP ready\r\n", &self.domain).as_bytes() ).await?;

        let mut line_buf = String::new();
        let mut session = SmtpSession::new();

        loop {
            line_buf.clear();
            if stream.read_line(&mut line_buf).await? == 0 {
                break;
            }

            if session.is_data {
                match line_buf.trim_end_matches(&['\r','\n'][..]) {
                    "." => {
                        session.is_data = false;
                        stream.write_all(b"250 Ok\r\n").await?;
                    },
                    "" => {
                        session.content.push('\n');
                    },
                    _ => {
                        session.content.push_str(
                            line_buf.trim_end_matches( &['\r','\n'][..] )
                        );
                        session.content.push('\n');
                    }
                }

                continue;
            }


            let mut line_iter = line_buf.split_whitespace();
            let command:&str = line_iter.next().unwrap_or("");

            match command.to_uppercase().as_str() {
                "STARTTLS" => {
                    if let Some(ref config) = self.config {
                        session.use_tls = true;
                        stream.write_all(b"220 2.0.0 Ready to start TLS\r\n").await?;
                        stream.start_tls(Arc::clone(config)).await?;
                    } else {
                        stream.write_all(format!("500 Unknown command: {}\r\n", command).as_bytes()).await?;
                    }
                },
                "EHLO" | "HELO" => {
                    if self.config.is_some() {
                        stream.write_all(b"250 STARTTLS\r\n").await?;
                    } else {
                        stream.write_all(b"250 Hello\r\n").await?;
                    }
                }
                "MAIL" => {
                    match extract_addresses(&line_buf) {
                        Some(addr) => {
                            session.from = addr.trim().to_string();
                            stream.write_all(b"250 Ok\r\n").await?;
                        }
                        None => {
                            stream.write_all(b"501 5.5.4 Syntax error in parameters\r\n").await?;
                        }
                    }
                }
                "RCPT" => {
                    match extract_addresses(&line_buf) {
                        Some(addr) => {
                            if self.is_local_recipient(addr) {
                                session.to.push(addr.trim().to_string());
                                stream.write_all(b"250 Ok\r\n").await?;
                            }else {
                                stream.write_all(b"550 5.7.1 Relay access denied\r\n").await?;
                            }
                        }
                        _ => {
                            stream.write_all(b"501 5.5.4 Syntax error in parameters\r\n").await?;
                        }
                    }
                }
                "DATA" => {
                    session.is_data = true;
                    stream.write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n").await?;
                }
                "QUIT" => {
                    session.quit = true;
                    stream.write_all(b"221 Bye\r\n").await?;
                    break;
                },
                _ => {
                    stream.write_all(format!("500 Unknown command: {}\r\n", command).as_bytes()).await?;
                }
            }
        }

        (self.post_handle)(session).await
    })}
}

impl Smtp
{
    pub fn new(domain:&str,
        post_handle:impl Fn(SmtpSession)-> AsyncConnectionFuture<'static, io::Result<usize>> + Send + Sync + 'static)
    -> Self
    {
        Self {
            domain: domain.to_string(),
            config: None,
            post_handle: Box::new(post_handle),
        }
    }

    pub fn set_config(&mut self, config:Arc<ServerConfig>) {
        self.config = Some(config);
    }

    fn is_local_recipient(&self, recipient: &str) -> bool
    {
        if let Some(at) = recipient.find('@') {
            return self.domain.eq_ignore_ascii_case(&recipient[at+1..]);
        }
        false
    }
}

fn extract_addresses(line: &str) -> Option<&str>
{
    let start = line.find('<')? + 1;
    let end = line[start..].find('>')? + start;
    Some(&line[start..end])
}
