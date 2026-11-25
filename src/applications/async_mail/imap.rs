use std::{io, sync::Arc};
use rustls::ServerConfig;
use crate::runtime::{AsyncProtocol, net::{AsyncConnectionFuture, AsyncTcpStream}};

pub struct Imap {
    config: Arc<ServerConfig>,
    login_service: Box<dyn Fn(&mut ImapSession) -> AsyncConnectionFuture<'static, io::Result<usize>> + Send + Sync>,
}

impl Imap {
    pub fn new(
        config: Arc<ServerConfig>,
        login_service: Box<dyn Fn(&mut ImapSession) -> AsyncConnectionFuture<'static, io::Result<usize>> + Send + Sync>,
    ) -> Self
    {
        Self {
            config,
            login_service
        }
    }
}

impl AsyncProtocol for Imap {
    fn handle_async_connection(&self, mut stream: AsyncTcpStream)
    -> AsyncConnectionFuture<'_, io::Result<usize>>
    {
        Box::pin(async move {
            stream.write_all( b"* OK IMAP4rev1 ready\r\n" ).await?;

            let mut session: ImapSession = ImapSession::new();
            let mut line_buf = String::new();

            loop {
                line_buf.clear();
                if stream.read_line(&mut line_buf).await? == 0 {break}
                if let Some(command) = Command::parse(&line_buf) {
                    match command.name() {
                        "CAPABILITY" => {},
                        "NOOP" => {},
                        "LOGOUT" => {},
                        "STARTTLS" => {},
                        "AUTHENTICATE" => {},
                        "LOGIN" => {},
                        "SELECT" => {},
                        "EXAMINE" => {},
                        "CREATE" => {},
                        "DELETE" => {},
                        "RENAME" => {},
                        "SUBSCRIBE" => {},
                        "UNSUBSCRIBE" => {},
                        "LIST" => {},
                        "LSUB" => {},
                        "STATUS" => {},
                        "APPEND" => {},
                        "CHECK" => {},
                        "CLOSE" => {},
                        "EXPUNGE" => {},
                        "SEARCH" => {},
                        "FETCH" => {},
                        "STORE" => {},
                        "COPY" => {},
                        "UID" => {},
                        "IDLE" => {},
                        "NAMESPACE" => {},
                        "UNSELECT" => {},
                        "MOVE" => {},
                        "ENABLE" => {},
                        "ID" => {},
                        _ => {}
                    }
                }else {
                    stream.write_all(b"* BAD Invalid command line").await?;
                }
            }

            Ok(1)
        })
    }
}

pub struct ImapSession {
    logged_in_as : Option<String>
}

impl ImapSession {
    fn new() -> Self {
        Self {
            logged_in_as: None
        }
    }
}

struct Command {
    tag: String,
    name: String,
    args: String
}

impl Command {
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim_end_matches(|t| t == '\r' || t == '\n');
        let (tag, rest) = line.split_once(' ')?;
        let (name, args) = rest.split_once(' ').unwrap_or((rest, ""));

        if tag.is_empty() || name.is_empty() {
            return None;
        }

        Some(Self {
            tag: tag.to_string(),
            name: name.to_ascii_uppercase(),
            args: args.to_string(),
        })
    }

    pub fn tag(&self) -> &str {
        self.tag.as_str()
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn args(&self) -> &str {
        self.args.as_str()
    }
}
