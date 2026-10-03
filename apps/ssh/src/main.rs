use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};
use russh::keys::{Algorithm, PrivateKey, ssh_key::LineEnding};
use russh::server::{Auth, Config, Handle, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId, CryptoVec, Pty};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

mod app;
mod journal;
mod keys;
mod markdown;
mod view;

use app::{App, Outcome};

const ENTER_SCREEN: &[u8] = b"\x1b[?1049h\x1b[?25l\x1b[2J";
const LEAVE_SCREEN: &[u8] = b"\x1b[2J\x1b[H\x1b[?25h\x1b[?1049l";

type SshTerminal = Terminal<CrosstermBackend<Outbound>>;

struct Outbound {
    sender: UnboundedSender<Vec<u8>>,
    buffer: Vec<u8>,
}

impl Outbound {
    fn start(handle: Handle, channel: ChannelId) -> Self {
        let (sender, mut receiver) = unbounded_channel::<Vec<u8>>();
        tokio::spawn(async move {
            while let Some(bytes) = receiver.recv().await {
                if handle.data(channel, CryptoVec::from(bytes)).await.is_err() {
                    break;
                }
            }
        });
        Self {
            sender,
            buffer: Vec::new(),
        }
    }
}

impl Write for Outbound {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.sender
            .send(std::mem::take(&mut self.buffer))
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "client gone"))
    }
}

struct Client {
    terminal: SshTerminal,
    app: App,
    area: Rect,
    started: bool,
}

impl Client {
    fn render(&mut self) {
        if !self.started || self.area.width == 0 || self.area.height == 0 {
            return;
        }
        let app = &mut self.app;
        let _ = self.terminal.draw(|frame| view::draw(frame, app));
    }

    fn write_raw(&mut self, bytes: &[u8]) {
        let backend = self.terminal.backend_mut();
        let _ = backend.write_all(bytes);
        let _ = backend.flush();
    }
}

#[derive(Clone)]
struct Portfolio {
    journal: journal::Shared,
    sessions: Arc<AtomicUsize>,
    limit: usize,
}

impl Server for Portfolio {
    type Handler = Connection;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Connection {
        Connection {
            shared: self.clone(),
            client: Arc::new(Mutex::new(None)),
            counted: false,
            turned_away: false,
        }
    }
}

struct Connection {
    shared: Portfolio,
    client: Arc<Mutex<Option<Client>>>,
    counted: bool,
    turned_away: bool,
}

impl Drop for Connection {
    fn drop(&mut self) {
        if self.counted {
            self.shared.sessions.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

fn resize(client: &mut Client, columns: u32, rows: u32) {
    let area = Rect::new(0, 0, columns.min(500) as u16, rows.min(300) as u16);
    client.area = area;
    let _ = client.terminal.resize(area);
}

impl Handler for Connection {
    type Error = russh::Error;

    async fn auth_none(&mut self, _: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(
        &mut self,
        _: &str,
        _: &russh::keys::PublicKey,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_password(&mut self, _: &str, _: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let mut slot = self.client.lock().await;
        if slot.is_some() {
            return Ok(false);
        }
        if !self.counted {
            let previous = self.shared.sessions.fetch_add(1, Ordering::SeqCst);
            self.counted = true;
            if previous >= self.shared.limit {
                self.turned_away = true;
                return Ok(true);
            }
        }
        let backend = CrosstermBackend::new(Outbound::start(session.handle(), channel.id()));
        let terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::default()),
            },
        )
        .map_err(|_| russh::Error::Disconnect)?;
        *slot = Some(Client {
            terminal,
            app: App::new(self.shared.journal.clone()),
            area: Rect::default(),
            started: false,
        });
        Ok(true)
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _: &str,
        columns: u32,
        rows: u32,
        _: u32,
        _: u32,
        _: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(client) = self.client.lock().await.as_mut() {
            resize(client, columns, rows);
        }
        session.channel_success(channel)?;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if self.turned_away {
            session.channel_success(channel)?;
            session.data(
                channel,
                CryptoVec::from_slice(
                    b"too many visitors right now, try again in a minute. life2harsh.xyz\r\n",
                ),
            )?;
            session.exit_status_request(channel, 1)?;
            session.eof(channel)?;
            session.close(channel)?;
            return Ok(());
        }
        {
            let mut slot = self.client.lock().await;
            let Some(client) = slot.as_mut() else {
                session.channel_failure(channel)?;
                return Ok(());
            };
            if client.area.width == 0 || client.area.height == 0 {
                resize(client, 80, 24);
            }
            client.started = true;
            client.write_raw(ENTER_SCREEN);
            client.render();
        }
        session.channel_success(channel)?;
        let client = self.client.clone();
        tokio::spawn(async move {
            let mut last = Instant::now();
            let mut ticker = tokio::time::interval(Duration::from_millis(80));
            loop {
                ticker.tick().await;
                let mut slot = client.lock().await;
                let Some(active) = slot.as_mut() else {
                    break;
                };
                let now = Instant::now();
                if active
                    .app
                    .tick(now.duration_since(last).as_secs_f64().min(0.25))
                {
                    active.render();
                }
                last = now;
            }
        });
        Ok(())
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        _: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_failure(channel)?;
        let _ = session.data(
            channel,
            CryptoVec::from_slice(
                b"this server only speaks interactive sessions: ssh -t life2harsh.xyz\r\n",
            ),
        );
        session.close(channel)?;
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        _: ChannelId,
        columns: u32,
        rows: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(client) = self.client.lock().await.as_mut() {
            resize(client, columns, rows);
            let _ = client.terminal.clear();
            client.render();
        }
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let mut slot = self.client.lock().await;
        let Some(client) = slot.as_mut() else {
            return Ok(());
        };
        for key in keys::parse(data) {
            if client.app.handle(key) == Outcome::Quit {
                client.write_raw(LEAVE_SCREEN);
                client.write_raw(b"thanks for stopping by. life2harsh.xyz\r\n");
                *slot = None;
                drop(slot);
                tokio::time::sleep(Duration::from_millis(50)).await;
                session.exit_status_request(channel, 0)?;
                session.eof(channel)?;
                session.close(channel)?;
                return Ok(());
            }
        }
        client.render();
        Ok(())
    }

    async fn channel_close(&mut self, _: ChannelId, _: &mut Session) -> Result<(), Self::Error> {
        self.client.lock().await.take();
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.client.lock().await.take();
        session.close(channel)?;
        Ok(())
    }
}

fn host_key(path: &str) -> PrivateKey {
    if let Ok(text) = std::fs::read_to_string(path)
        && let Ok(key) = PrivateKey::from_openssh(&text)
    {
        return key;
    }
    let key =
        PrivateKey::random(&mut rand_core::OsRng, Algorithm::Ed25519).expect("generate host key");
    if let Ok(text) = key.to_openssh(LineEnding::LF) {
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode_private()
            .open(path)
            .and_then(|mut file| file.write_all(text.as_bytes()));
        if let Err(cause) = written {
            eprintln!("could not persist host key at {path}: {cause}");
        }
    }
    key
}

trait PrivateMode {
    fn mode_private(&mut self) -> &mut Self;
}

impl PrivateMode for std::fs::OpenOptions {
    #[cfg(unix)]
    fn mode_private(&mut self) -> &mut Self {
        use std::os::unix::fs::OpenOptionsExt;
        self.mode(0o600)
    }

    #[cfg(not(unix))]
    fn mode_private(&mut self) -> &mut Self {
        self
    }
}

#[tokio::main]
async fn main() {
    let bind = std::env::var("SSH_BIND").unwrap_or_else(|_| "0.0.0.0:2222".into());
    let key_path = std::env::var("SSH_HOST_KEY").unwrap_or_else(|_| "ssh_host_ed25519_key".into());
    let journal_address = std::env::var("SSH_JOURNAL").unwrap_or_else(|_| "127.0.0.1:7841".into());
    let limit = std::env::var("SSH_MAX_SESSIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(24);

    let journal = journal::shared();
    journal::spawn_refresher(journal.clone(), journal_address);

    let config = Arc::new(Config {
        inactivity_timeout: Some(Duration::from_secs(15 * 60)),
        auth_rejection_time: Duration::from_millis(300),
        auth_rejection_time_initial: Some(Duration::from_secs(0)),
        keys: vec![host_key(&key_path)],
        nodelay: true,
        ..Default::default()
    });
    let mut server = Portfolio {
        journal,
        sessions: Arc::new(AtomicUsize::new(0)),
        limit,
    };
    eprintln!("ssh portfolio listening on {bind}");
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    tokio::select! {
        result = server.run_on_address(config, bind.as_str()) => {
            if let Err(cause) = result {
                eprintln!("server error: {cause}");
                std::process::exit(1);
            }
        }
        _ = shutdown => {}
    }
}
