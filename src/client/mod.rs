//! Thin client mode — connects to the server's client socket.
//!
//! The client:
//! - Connects to `herdr-client.sock`, sends Hello with terminal size and protocol version
//! - Sets up the real terminal (raw mode, mouse capture, keyboard enhancements)
//! - Receives Frame messages and blits them to the terminal (diff against last frame)
//! - Reads stdin events (keystrokes, mouse, paste) and sends them as ClientMessage::Input
//! - Detects terminal resize and sends ClientMessage::Resize
//! - Restores terminal on exit (normal or error)
//! - Handles ServerShutdown gracefully (clean exit, informative message to stderr)
//! - Handles server unreachable (clear error screen, not blank/hang)
//! - Forwards OSC 52 clipboard writes from server to its own stdout
//! - Displays sound/toast notifications forwarded from server

mod input;

use std::collections::HashSet;
use std::io::{self, Write as _};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use tracing::{debug, info, warn};

use crate::protocol::render_ansi;
use crate::protocol::{
    self, AttachScrollDirection, AttachScrollSource, ClientKeybindings, ClientMessage, NotifyKind,
    RenderEncoding, ServerMessage, MAX_CLIPBOARD_IMAGE_PAYLOAD, MAX_FRAME_SIZE,
    MAX_GRAPHICS_FRAME_SIZE, PROTOCOL_VERSION,
};
use crate::server::socket_paths::client_socket_path;

static RECEIVED_KITTY_GRAPHICS_IDS: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();

#[allow(dead_code)]
struct SendStream(cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

// ---------------------------------------------------------------------------
// Client state
// ---------------------------------------------------------------------------

/// State tracking for the thin client.
struct ClientState {
    /// Stateful semantic-frame encoder used when the server sends FrameData.
    blit_encoder: render_ansi::BlitEncoder,
    /// Whether host mouse capture is currently active.
    mouse_capture_active: bool,
    /// The terminal size we reported to the server in our last Hello/Resize.
    reported_size: (u16, u16),
    /// Client-local sound playback config, refreshed on server request.
    sound_config: crate::config::SoundConfig,
    /// Whether this client may write Kitty graphics bytes to its host terminal.
    kitty_graphics_enabled: bool,
    /// Direct attach prefix escape state. None for full-app clients.
    attach_escape: Option<AttachEscapeState>,
    /// Rows scrolled for one direct-attach wheel notch.
    mouse_scroll_lines: usize,
    /// Whether outer focus gain should force a full host-terminal redraw.
    redraw_on_focus_gained: bool,
    /// CPAL recording stream to keep it alive.
    recording_stream: Option<SendStream>,
    /// Flags for signaling the recording task.
    recording_active: Option<Arc<std::sync::atomic::AtomicBool>>,
    recording_aborted: Option<Arc<std::sync::atomic::AtomicBool>>,
    recording_buffer: Option<Arc<std::sync::Mutex<Vec<f32>>>>,
}

#[derive(Debug, Default)]
struct AttachEscapeState {
    pending_prefix: bool,
}

#[derive(Debug)]
enum AttachInputAction {
    Forward(Vec<u8>),
    Scroll {
        source: AttachScrollSource,
        direction: AttachScrollDirection,
        lines: u16,
        column: Option<u16>,
        row: Option<u16>,
        modifiers: u8,
    },
    Detach,
    None,
}

impl AttachEscapeState {
    fn filter_input(
        &mut self,
        data: Vec<u8>,
        viewport_rows: u16,
        mouse_scroll_lines: usize,
    ) -> AttachInputAction {
        const PREFIX: u8 = 0x02; // Ctrl+B

        let mut output = Vec::with_capacity(data.len());
        for byte in data {
            if self.pending_prefix {
                self.pending_prefix = false;
                match byte {
                    b'q' => return AttachInputAction::Detach,
                    PREFIX => output.push(PREFIX),
                    other => {
                        output.push(PREFIX);
                        output.push(other);
                    }
                }
                continue;
            }

            if byte == PREFIX {
                self.pending_prefix = true;
            } else {
                output.push(byte);
            }
        }

        if output.is_empty() {
            AttachInputAction::None
        } else if let Some(action) =
            attach_scroll_action(&output, viewport_rows, mouse_scroll_lines)
        {
            action
        } else {
            AttachInputAction::Forward(output)
        }
    }
}

fn attach_scroll_action(
    data: &[u8],
    viewport_rows: u16,
    mouse_scroll_lines: usize,
) -> Option<AttachInputAction> {
    let mut events = crate::raw_input::parse_raw_input_bytes_sync(data);
    if events.len() != 1 {
        return None;
    }

    match events.pop()? {
        crate::raw_input::RawInputEvent::Mouse(mouse) => {
            let direction = match mouse.kind {
                MouseEventKind::ScrollUp => AttachScrollDirection::Up,
                MouseEventKind::ScrollDown => AttachScrollDirection::Down,
                _ => return Some(AttachInputAction::None),
            };
            Some(AttachInputAction::Scroll {
                source: AttachScrollSource::Wheel,
                direction,
                lines: mouse_scroll_lines.max(1).min(u16::MAX as usize) as u16,
                column: Some(mouse.column),
                row: Some(mouse.row),
                modifiers: mouse.modifiers.bits(),
            })
        }
        crate::raw_input::RawInputEvent::Key(key)
            if key.modifiers.is_empty()
                && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
        {
            let direction = match key.code {
                KeyCode::PageUp => AttachScrollDirection::Up,
                KeyCode::PageDown => AttachScrollDirection::Down,
                _ => return None,
            };
            Some(AttachInputAction::Scroll {
                source: AttachScrollSource::PageKey {
                    input: data.to_vec(),
                },
                direction,
                lines: viewport_rows.saturating_sub(1).max(1),
                column: None,
                row: None,
                modifiers: KeyModifiers::empty().bits(),
            })
        }
        crate::raw_input::RawInputEvent::Key(key)
            if key.modifiers.is_empty()
                && key.kind == KeyEventKind::Release
                && matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) =>
        {
            Some(AttachInputAction::None)
        }
        _ => None,
    }
}

impl ClientState {
    fn request_full_redraw(&mut self) {
        self.blit_encoder = render_ansi::BlitEncoder::new();
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during client operation.
#[derive(Debug)]
pub enum ClientError {
    /// Could not connect to the server's client socket.
    ConnectionFailed(io::Error),
    /// Server rejected our handshake.
    HandshakeRejected { version: u32, error: String },
    /// Server shut down.
    ServerShutdown { reason: Option<String> },
    /// Lost connection to the server.
    ConnectionLost(io::Error),
    /// Protocol error (framing, deserialization).
    Protocol(protocol::FramingError),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::ConnectionFailed(err) => {
                write!(f, "failed to connect to server: {err}")?;
                let path = client_socket_path();
                write!(
                    f,
                    "\nIs herdr server running? Start it with `herdr server`."
                )?;
                write!(f, "\nSocket path: {}", path.display())
            }
            ClientError::HandshakeRejected { version, error } => {
                write!(f, "server rejected handshake (version {version}): {error}")
            }
            ClientError::ServerShutdown { reason } => {
                match reason.as_deref() {
                    Some("detached") => {
                        if let Ok(reattach_command) =
                            std::env::var(crate::remote::REATTACH_COMMAND_ENV_VAR)
                        {
                            write!(f, "detached from remote server")?;
                            write!(f, "\nRun `{reattach_command}` to reattach")?;
                        } else {
                            write!(f, "detached from server")?;
                            write!(
                                f,
                                "\nRun `{}` to reattach",
                                crate::session::local_attach_command()
                            )?;
                        }
                    }
                    _ => {
                        write!(f, "server shut down")?;
                        if let Some(reason) = reason {
                            write!(f, ": {reason}")?;
                        }
                    }
                }
                Ok(())
            }
            ClientError::ConnectionLost(err) => {
                write!(f, "lost connection to server: {err}")
            }
            ClientError::Protocol(err) => {
                write!(f, "protocol error: {err}")
            }
        }
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClientError::ConnectionFailed(err) => Some(err),
            ClientError::ConnectionLost(err) => Some(err),
            ClientError::Protocol(err) => Some(err),
            _ => None,
        }
    }
}

impl From<protocol::FramingError> for ClientError {
    fn from(err: protocol::FramingError) -> Self {
        ClientError::Protocol(err)
    }
}

// ---------------------------------------------------------------------------
// Terminal setup / restore
// ---------------------------------------------------------------------------

/// Sets up the terminal for client mode (raw mode, optional mouse, keyboard enhancements).
///
/// Returns a guard that restores the terminal when dropped.
fn setup_terminal(mouse_capture: bool) -> io::Result<TerminalGuard> {
    setup_terminal_with_capabilities(true, mouse_capture)
}

/// Sets up a direct attach terminal.
///
/// Direct attach forwards stdin to the attached PTY. It enables mouse capture
/// so wheel events can drive the attached viewport or be forwarded to child
/// programs that requested mouse input.
fn setup_direct_attach_terminal() -> io::Result<TerminalGuard> {
    setup_terminal_with_capabilities(false, true)
}

fn setup_terminal_with_capabilities(
    enable_client_protocols: bool,
    mouse_capture: bool,
) -> io::Result<TerminalGuard> {
    ratatui::init();

    if enable_client_protocols {
        if mouse_capture {
            execute!(io::stdout(), EnableMouseCapture)?;
        } else {
            execute!(io::stdout(), DisableMouseCapture)?;
        }
        execute!(
            io::stdout(),
            EnableBracketedPaste,
            EnableFocusChange,
            PushKeyboardEnhancementFlags(crate::input::ime_compatible_keyboard_enhancement_flags())
        )?;
    } else if mouse_capture {
        execute!(io::stdout(), EnableMouseCapture)?;
    } else {
        execute!(io::stdout(), DisableMouseCapture)?;
    }

    let modify_other_keys_mode = enable_client_protocols
        .then(|| {
            crate::input::host_modify_other_keys_mode(
                std::env::var("TMUX").is_ok(),
                std::env::var("TERM_PROGRAM").ok().as_deref(),
                std::env::var_os("WEZTERM_PANE").is_some(),
            )
        })
        .flatten();
    if let Some(mode) = modify_other_keys_mode {
        io::stdout().write_all(mode.set_sequence())?;
        io::stdout().flush()?;
    }

    Ok(TerminalGuard {
        reset_modify_other_keys: modify_other_keys_mode.is_some(),
    })
}

/// Guard that restores the terminal when dropped.
struct TerminalGuard {
    reset_modify_other_keys: bool,
}

fn write_terminal_restore_postlude(writer: &mut impl io::Write) -> io::Result<()> {
    // Restore a visible cursor and reset DECSCUSR back to the terminal default.
    writer.write_all(b"\x1b[?25h\x1b[0 q")?;
    writer.flush()
}

fn set_mouse_capture(enabled: bool) -> io::Result<()> {
    if enabled {
        execute!(io::stdout(), EnableMouseCapture)
    } else {
        execute!(io::stdout(), DisableMouseCapture)
    }
}

fn restore_terminal_state(reset_modify_other_keys: bool) {
    let _ = clear_received_kitty_graphics(&mut io::stdout());

    // Reset modifyOtherKeys if we enabled it.
    if reset_modify_other_keys {
        let _ = io::stdout().write_all(b"\x1b[>4;0m");
        let _ = io::stdout().flush();
    }

    let _ = execute!(
        io::stdout(),
        PopKeyboardEnhancementFlags,
        DisableFocusChange,
        DisableBracketedPaste,
        DisableMouseCapture
    );
    ratatui::restore();
    let _ = write_terminal_restore_postlude(&mut io::stdout());
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal_state(self.reset_modify_other_keys);
    }
}

// ---------------------------------------------------------------------------
// Handshake
// ---------------------------------------------------------------------------

fn requested_render_encoding() -> RenderEncoding {
    match std::env::var("HERDR_RENDER_ENCODING").ok().as_deref() {
        Some("terminal-ansi" | "terminal_ansi" | "ansi") => RenderEncoding::TerminalAnsi,
        _ => RenderEncoding::SemanticFrame,
    }
}

fn requested_keybindings() -> ClientKeybindings {
    match std::env::var(crate::remote::REMOTE_KEYBINDINGS_ENV_VAR)
        .ok()
        .as_deref()
    {
        Some("local") => crate::config::Config::load()
            .config
            .local_keybindings_profile_toml()
            .map(|keys_toml| ClientKeybindings::Local { keys_toml })
            .unwrap_or(ClientKeybindings::Server),
        _ => ClientKeybindings::Server,
    }
}

/// Performs the client→server handshake.
///
/// Sends Hello with the terminal size and protocol version, reads the Welcome
/// response. Returns Ok(()) on success, or an error if the server rejects us.
fn do_handshake(
    stream: &mut UnixStream,
    cols: u16,
    rows: u16,
    cell_width_px: u32,
    cell_height_px: u32,
    requested_encoding: RenderEncoding,
) -> Result<RenderEncoding, ClientError> {
    stream
        .set_nonblocking(false)
        .map_err(ClientError::ConnectionFailed)?;

    // Send Hello.
    let hello = ClientMessage::Hello {
        version: PROTOCOL_VERSION,
        cols,
        rows,
        cell_width_px,
        cell_height_px,
        requested_encoding,
        keybindings: requested_keybindings(),
    };
    protocol::write_message(stream, &hello)
        .map_err(|e| ClientError::ConnectionFailed(io::Error::other(e.to_string())))?;

    // Read Welcome.
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(ClientError::ConnectionFailed)?;
    let welcome: ServerMessage = protocol::read_message(stream, MAX_FRAME_SIZE)?;
    stream
        .set_read_timeout(None)
        .map_err(ClientError::ConnectionFailed)?;

    match welcome {
        ServerMessage::Welcome {
            version,
            encoding,
            error,
        } => {
            if let Some(error) = error {
                return Err(ClientError::HandshakeRejected { version, error });
            }
            info!(version, ?encoding, "handshake succeeded");
            Ok(encoding)
        }
        _ => Err(ClientError::Protocol(protocol::FramingError::Io(
            io::Error::new(io::ErrorKind::InvalidData, "expected Welcome message"),
        ))),
    }
}

// ---------------------------------------------------------------------------
// Client event loop
// ---------------------------------------------------------------------------

/// Internal events for the client event loop.
enum ClientLoopEvent {
    /// Raw input bytes from stdin.
    StdinInput(Vec<u8>),
    /// Terminal resize detected.
    Resize(u16, u16, u32, u32),
    /// Server message received.
    ServerMessage(ServerMessage),
    /// Server reader thread exited (connection lost).
    ServerDisconnected,
    /// Timer tick.
    Timer,
    /// Audio stream started.
    RecordingStarted(SendStream),
    /// Background client message to send to server.
    ClientMessageToSend(ClientMessage),
}

/// Runs the thin client: connects to the server, performs the handshake,
/// and enters the main event loop.
///
/// This is the entry point called from `main.rs` when running in client mode.
pub fn run_client() -> io::Result<()> {
    run_client_with_mode(
        requested_render_encoding(),
        None,
        None,
        "connecting to server",
    )
}

/// Runs a direct terminal attach client.
pub fn run_terminal_attach(terminal_id: String, takeover: bool) -> io::Result<()> {
    run_client_with_mode(
        RenderEncoding::TerminalAnsi,
        Some((terminal_id, takeover)),
        Some(AttachEscapeState::default()),
        "attaching to terminal",
    )
}

fn run_client_with_mode(
    requested_encoding: RenderEncoding,
    attach_request: Option<(String, bool)>,
    attach_escape: Option<AttachEscapeState>,
    log_message: &'static str,
) -> io::Result<()> {
    init_logging();

    let loaded_config = crate::config::Config::load();
    let mouse_scroll_lines = loaded_config.config.ui.mouse_scroll_lines();
    let redraw_on_focus_gained = loaded_config.config.ui.redraw_on_focus_gained;
    let sound_config = loaded_config.config.ui.sound;
    let direct_attach_requested = attach_request.is_some();
    let kitty_graphics_enabled =
        loaded_config.config.experimental.kitty_graphics && !direct_attach_requested;

    let socket_path = client_socket_path();
    crate::logging::startup("client");
    info!(path = %socket_path.display(), "{log_message}");

    // Try to connect to the server.
    let mut stream = match UnixStream::connect(&socket_path) {
        Ok(s) => s,
        Err(err) => {
            // Server unreachable — show clear error and exit.
            let client_err = ClientError::ConnectionFailed(err);
            eprintln!("herdr: {client_err}");
            std::process::exit(1);
        }
    };

    // Get the terminal geometry before handshake (before raw mode).
    let (cols, rows, cell_width_px, cell_height_px) =
        current_terminal_geometry(kitty_graphics_enabled);

    // Perform handshake while the stream is still in blocking mode.
    let negotiated_encoding = match do_handshake(
        &mut stream,
        cols,
        rows,
        cell_width_px,
        cell_height_px,
        requested_encoding,
    ) {
        Ok(encoding) => encoding,
        Err(err) => {
            eprintln!("herdr: {err}");
            std::process::exit(1);
        }
    };

    if let Some((terminal_id, takeover)) = attach_request {
        let attach = ClientMessage::AttachTerminal {
            terminal_id,
            takeover,
        };
        if let Err(err) = write_to_server(&mut stream, &attach) {
            eprintln!("herdr: failed to request terminal attach: {err}");
            std::process::exit(1);
        }
    }

    // Now set up the terminal. This must happen AFTER the handshake succeeds,
    // so we don't leave the terminal in raw mode if the server rejects us.
    let direct_attach = attach_escape.is_some();
    let _guard = if direct_attach {
        setup_direct_attach_terminal()
    } else {
        setup_terminal(false)
    }
    .map_err(|err| {
        eprintln!("herdr: failed to set up terminal: {err}");
        err
    })?;

    // Install a panic hook to restore the terminal on panic (same as monolithic).
    let in_tmux = std::env::var("TMUX").is_ok();
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal_state(in_tmux);
        original_hook(info);
    }));

    // Create the tokio runtime.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(io::Error::other)?;

    let should_quit = Arc::new(AtomicBool::new(false));

    // Install Ctrl+C handler.
    let quit_flag = should_quit.clone();
    let _ = ctrlc::set_handler(move || {
        quit_flag.store(true, Ordering::Release);
    });

    let result = rt.block_on(async {
        run_client_loop(
            stream,
            cols,
            rows,
            should_quit,
            sound_config,
            mouse_scroll_lines,
            redraw_on_focus_gained,
            kitty_graphics_enabled,
            false,
            negotiated_encoding,
            attach_escape,
        )
        .await
    });

    // Restore the terminal before printing any final status message.
    drop(_guard);

    if let Err(err) = result {
        eprintln!("herdr: {err}");
        rt.shutdown_timeout(Duration::from_millis(100));
        crate::logging::shutdown("client");

        if matches!(
            err,
            ClientError::ServerShutdown {
                reason: Some(reason)
            } if reason == "detached"
        ) {
            return Ok(());
        }

        std::process::exit(1);
    }

    rt.shutdown_timeout(Duration::from_millis(100));
    crate::logging::shutdown("client");
    Ok(())
}

/// The main client event loop.
///
/// Uses a threaded architecture:
/// - stdin reader thread → sends raw input bytes to main loop
/// - resize poller thread → sends resize events to main loop
/// - server reader thread → reads ServerMessages and sends to main loop
/// - main loop: coordinates input, output, and server communication
async fn run_client_loop(
    stream: UnixStream,
    cols: u16,
    rows: u16,
    should_quit: Arc<AtomicBool>,
    sound_config: crate::config::SoundConfig,
    mouse_scroll_lines: usize,
    redraw_on_focus_gained: bool,
    kitty_graphics_enabled: bool,
    mouse_capture_active: bool,
    negotiated_encoding: RenderEncoding,
    attach_escape: Option<AttachEscapeState>,
) -> Result<(), ClientError> {
    let mut state = ClientState {
        blit_encoder: render_ansi::BlitEncoder::new(),
        mouse_capture_active,
        reported_size: (cols, rows),
        sound_config,
        kitty_graphics_enabled,
        attach_escape,
        mouse_scroll_lines,
        redraw_on_focus_gained,
        recording_stream: None,
        recording_active: None,
        recording_aborted: None,
        recording_buffer: None,
    };
    debug!(?negotiated_encoding, "client render encoding active");

    // Channel for events from the stdin, resize, and server reader threads.
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<ClientLoopEvent>(256);

    // Spawn the stdin reader thread.
    let stdin_quit = should_quit.clone();
    let stdin_tx = event_tx.clone();
    std::thread::spawn(move || {
        input::stdin_reader_loop(stdin_tx, &stdin_quit);
    });

    if state.attach_escape.is_none() {
        query_host_terminal_theme();
    }

    // Spawn the resize poller thread.
    let resize_quit = should_quit.clone();
    let resize_tx = event_tx.clone();
    std::thread::spawn(move || {
        resize_poll_loop(resize_tx, cols, rows, kitty_graphics_enabled, &resize_quit);
    });

    // Spawn the server reader thread (blocking reads from the socket).
    // Clone the stream's file descriptor so we can read from a blocking stream.
    let server_read_quit = should_quit.clone();
    let server_read_tx = event_tx.clone();
    let read_stream = stream.try_clone().map_err(ClientError::ConnectionFailed)?;
    std::thread::spawn(move || {
        let max_frame_size = if kitty_graphics_enabled {
            MAX_GRAPHICS_FRAME_SIZE
        } else {
            MAX_FRAME_SIZE
        };
        server_reader_thread(
            read_stream,
            server_read_tx,
            &server_read_quit,
            max_frame_size,
        );
    });

    // Use the original stream for writing (blocking is fine since we write
    // from the async loop).
    let mut write_stream = stream;
    write_stream
        .set_nonblocking(false)
        .map_err(ClientError::ConnectionFailed)?;

    // Main event loop.
    while !should_quit.load(Ordering::Acquire) {
        let event = tokio::select! {
            ev = event_rx.recv() => ev.unwrap_or(ClientLoopEvent::Timer),
            _ = tokio::time::sleep(Duration::from_millis(100)) => ClientLoopEvent::Timer,
        };

        match event {
            ClientLoopEvent::StdinInput(data) => {
                let data = if let Some(attach_escape) = &mut state.attach_escape {
                    match attach_escape.filter_input(
                        data,
                        state.reported_size.1,
                        state.mouse_scroll_lines,
                    ) {
                        AttachInputAction::Forward(data) => data,
                        AttachInputAction::Scroll {
                            source,
                            direction,
                            lines,
                            column,
                            row,
                            modifiers,
                        } => {
                            let msg = ClientMessage::AttachScroll {
                                source,
                                direction,
                                lines,
                                column,
                                row,
                                modifiers,
                            };
                            if let Err(e) = write_to_server(&mut write_stream, &msg) {
                                return Err(ClientError::ConnectionLost(e));
                            }
                            continue;
                        }
                        AttachInputAction::Detach => {
                            let _ = write_to_server(&mut write_stream, &ClientMessage::Detach);
                            return Ok(());
                        }
                        AttachInputAction::None => continue,
                    }
                } else {
                    let events = crate::raw_input::parse_raw_input_bytes_sync(&data);
                    if crate::raw_input::events_require_host_surface_redraw(
                        &events,
                        state.redraw_on_focus_gained,
                    ) {
                        state.request_full_redraw();
                    }
                    data
                };
                if should_bridge_clipboard_image_paste(&data) {
                    if let Some(image) = crate::platform::read_clipboard_image() {
                        if image.bytes.len() > MAX_CLIPBOARD_IMAGE_PAYLOAD {
                            warn!(
                                bytes = image.bytes.len(),
                                max = MAX_CLIPBOARD_IMAGE_PAYLOAD,
                                "local clipboard image is too large to bridge"
                            );
                            continue;
                        }
                        info!(
                            bytes = image.bytes.len(),
                            extension = image.extension,
                            "bridging local clipboard image paste to remote server"
                        );
                        let msg = ClientMessage::ClipboardImage {
                            extension: image.extension.to_owned(),
                            data: image.bytes,
                        };
                        if let Err(e) = write_to_server(&mut write_stream, &msg) {
                            return Err(ClientError::ConnectionLost(e));
                        }
                        continue;
                    }
                    info!(
                        "clipboard image paste trigger received, but local clipboard has no image"
                    );
                }
                let msg = ClientMessage::Input { data };
                if let Err(e) = write_to_server(&mut write_stream, &msg) {
                    return Err(ClientError::ConnectionLost(e));
                }
            }
            ClientLoopEvent::Resize(new_cols, new_rows, cell_width_px, cell_height_px) => {
                state.reported_size = (new_cols, new_rows);
                let msg = ClientMessage::Resize {
                    cols: new_cols,
                    rows: new_rows,
                    cell_width_px,
                    cell_height_px,
                };
                if let Err(e) = write_to_server(&mut write_stream, &msg) {
                    return Err(ClientError::ConnectionLost(e));
                }
            }
            ClientLoopEvent::ServerMessage(msg) => match msg {
                ServerMessage::Frame(frame_data) => {
                    let encoded = state.blit_encoder.encode(&frame_data, false);
                    let mut stdout = io::stdout();
                    let graphics = if state.kitty_graphics_enabled {
                        frame_data.graphics.as_slice()
                    } else {
                        &[]
                    };
                    let _ =
                        write_encoded_frame_with_graphics(&mut stdout, &encoded.bytes, graphics);
                    let _ = stdout.flush();
                    state.blit_encoder.commit(frame_data, encoded);
                }
                ServerMessage::Terminal(frame) => {
                    if state.kitty_graphics_enabled && contains_kitty_graphics_bytes(&frame.bytes) {
                        record_received_kitty_graphics(&frame.bytes);
                    }
                    let mut stdout = io::stdout();
                    let _ = stdout.write_all(&frame.bytes);
                    let _ = stdout.flush();
                }
                ServerMessage::Graphics { bytes } => {
                    if state.kitty_graphics_enabled {
                        record_received_kitty_graphics(&bytes);
                        let mut stdout = io::stdout();
                        let _ = stdout.write_all(&bytes);
                        let _ = stdout.flush();
                    }
                }
                ServerMessage::ServerShutdown { reason } => {
                    return Err(ClientError::ServerShutdown { reason });
                }
                ServerMessage::Notify { kind, message } => {
                    handle_notify(kind, &message, &state.sound_config);
                }
                ServerMessage::Clipboard { data } => {
                    forward_clipboard(&data);
                    let _ = io::stdout().flush();
                }
                ServerMessage::ReloadSoundConfig => {
                    reload_local_client_config(
                        &mut state.sound_config,
                        &mut state.redraw_on_focus_gained,
                    );
                }
                ServerMessage::MouseCapture { enabled } => {
                    let desired = enabled;
                    if desired != state.mouse_capture_active {
                        set_mouse_capture(desired).map_err(ClientError::ConnectionFailed)?;
                        state.mouse_capture_active = desired;
                    }
                }
                ServerMessage::Welcome { .. } => {
                    debug!("received unexpected Welcome in main loop");
                }
                ServerMessage::StartRecording {
                    workspace_id,
                    pane_id,
                    is_agent,
                } => {
                    // Stop any existing recording
                    if let Some(active) = state.recording_active.take() {
                        active.store(false, Ordering::Release);
                    }
                    if let Some(aborted) = state.recording_aborted.take() {
                        aborted.store(true, Ordering::Release);
                    }
                    state.recording_stream = None;
                    state.recording_buffer = None;

                    // Load config
                    let loaded_config = match crate::config::load_live_config() {
                        Ok(loaded) => loaded,
                        Err(errs) => {
                            let err_msg =
                                format!("Failed to load local config: {}", errs.join("; "));
                            let error_msg = ClientMessage::SpeechTranscribed {
                                workspace_id: workspace_id.clone(),
                                pane_id,
                                result: Err(err_msg),
                            };
                            if let Err(e) = write_to_server(&mut write_stream, &error_msg) {
                                return Err(ClientError::ConnectionLost(e));
                            }
                            continue;
                        }
                    };

                    let api_key = match loaded_config.config.speech_to_text.gemini_api_key.as_ref()
                    {
                        Some(k) if !k.trim().is_empty() => k.clone(),
                        _ => {
                            let error_msg = ClientMessage::SpeechTranscribed {
                                workspace_id: workspace_id.clone(),
                                pane_id,
                                result: Err("Gemini API key is not configured in local config (~/.config/herdr/config.toml).".to_string()),
                            };
                            if let Err(e) = write_to_server(&mut write_stream, &error_msg) {
                                return Err(ClientError::ConnectionLost(e));
                            }
                            continue;
                        }
                    };

                    let model = loaded_config
                        .config
                        .speech_to_text
                        .model
                        .clone()
                        .unwrap_or_else(|| "gemini-3.1-flash-live-preview".to_string());

                    let system_instruction = if is_agent {
                        loaded_config
                            .config
                            .speech_to_text
                            .agent_system_instruction
                            .clone()
                            .or_else(|| loaded_config.config.speech_to_text.system_instruction.clone())
                    } else {
                        loaded_config
                            .config
                            .speech_to_text
                            .terminal_system_instruction
                            .clone()
                            .or_else(|| loaded_config.config.speech_to_text.system_instruction.clone())
                    }
                    .unwrap_or_else(|| "You are a transcription engine. Output the exact text of the audio you hear. Do not converse, do not answer questions, and do not add commentary.".to_string());

                    let recording_active = Arc::new(std::sync::atomic::AtomicBool::new(true));
                    let recording_aborted = Arc::new(std::sync::atomic::AtomicBool::new(false));
                    let buffer = Arc::new(std::sync::Mutex::new(Vec::new()));

                    state.recording_active = Some(recording_active.clone());
                    state.recording_aborted = Some(recording_aborted.clone());
                    state.recording_buffer = Some(buffer.clone());

                    let event_tx_clone = event_tx.clone();
                    let api_key_clone = api_key.clone();
                    let workspace_id_clone = workspace_id.clone();
                    let buffer_clone = buffer.clone();
                    let recording_active_clone = recording_active.clone();
                    let recording_aborted_clone = recording_aborted.clone();

                    let tokio_handle = tokio::runtime::Handle::current();

                    std::thread::spawn(move || {
                        let host = cpal::default_host();
                        let device = match host.default_input_device() {
                            Some(dev) => dev,
                            None => {
                                let _ = event_tx_clone.blocking_send(
                                    ClientLoopEvent::ClientMessageToSend(
                                        ClientMessage::SpeechTranscribed {
                                            workspace_id: workspace_id_clone,
                                            pane_id,
                                            result: Err("No default input device found on client."
                                                .to_string()),
                                        },
                                    ),
                                );
                                return;
                            }
                        };

                        let config = match device.default_input_config() {
                            Ok(cfg) => cfg,
                            Err(e) => {
                                let _ = event_tx_clone.blocking_send(
                                    ClientLoopEvent::ClientMessageToSend(
                                        ClientMessage::SpeechTranscribed {
                                            workspace_id: workspace_id_clone,
                                            pane_id,
                                            result: Err(format!(
                                                "Failed to get client input config: {}",
                                                e
                                            )),
                                        },
                                    ),
                                );
                                return;
                            }
                        };

                        let sample_rate = config.sample_rate().0;
                        let channels = config.channels();
                        let sample_format = config.sample_format();

                        let buffer_writer = buffer_clone.clone();
                        let err_fn = move |err| {
                            tracing::error!("Audio recording stream error: {}", err);
                        };

                        fn write_input_data_client<T>(
                            data: &[T],
                            buffer: &Arc<std::sync::Mutex<Vec<f32>>>,
                        ) where
                            T: cpal::Sample<Float = f32>,
                        {
                            if let Ok(mut buf) = buffer.lock() {
                                for &sample in data {
                                    buf.push(sample.to_float_sample());
                                }
                            }
                        }

                        let stream = match sample_format {
                            cpal::SampleFormat::I16 => device.build_input_stream(
                                &config.into(),
                                move |data: &[i16], _: &_| {
                                    write_input_data_client(data, &buffer_writer);
                                },
                                err_fn,
                                None,
                            ),
                            cpal::SampleFormat::U16 => device.build_input_stream(
                                &config.into(),
                                move |data: &[u16], _: &_| {
                                    write_input_data_client(data, &buffer_writer);
                                },
                                err_fn,
                                None,
                            ),
                            cpal::SampleFormat::F32 => device.build_input_stream(
                                &config.into(),
                                move |data: &[f32], _: &_| {
                                    write_input_data_client(data, &buffer_writer);
                                },
                                err_fn,
                                None,
                            ),
                            _ => {
                                let _ = event_tx_clone.blocking_send(
                                    ClientLoopEvent::ClientMessageToSend(
                                        ClientMessage::SpeechTranscribed {
                                            workspace_id: workspace_id_clone,
                                            pane_id,
                                            result: Err(
                                                "Unsupported sample format on client.".to_string()
                                            ),
                                        },
                                    ),
                                );
                                return;
                            }
                        };

                        let stream = match stream {
                            Ok(s) => s,
                            Err(e) => {
                                let _ = event_tx_clone.blocking_send(
                                    ClientLoopEvent::ClientMessageToSend(
                                        ClientMessage::SpeechTranscribed {
                                            workspace_id: workspace_id_clone,
                                            pane_id,
                                            result: Err(format!(
                                                "Failed to build input stream on client: {}",
                                                e
                                            )),
                                        },
                                    ),
                                );
                                return;
                            }
                        };

                        if let Err(e) = stream.play() {
                            let _ =
                                event_tx_clone.blocking_send(ClientLoopEvent::ClientMessageToSend(
                                    ClientMessage::SpeechTranscribed {
                                        workspace_id: workspace_id_clone,
                                        pane_id,
                                        result: Err(format!(
                                            "Failed to play input stream on client: {}",
                                            e
                                        )),
                                    },
                                ));
                            return;
                        }

                        if event_tx_clone
                            .blocking_send(ClientLoopEvent::RecordingStarted(SendStream(stream)))
                            .is_err()
                        {
                            return;
                        }

                        tokio_handle.spawn(run_client_websocket_transcription(
                            workspace_id_clone,
                            pane_id,
                            api_key_clone,
                            model,
                            system_instruction,
                            buffer_clone,
                            channels,
                            sample_rate,
                            recording_active_clone,
                            recording_aborted_clone,
                            event_tx_clone,
                        ));
                    });
                }
                ServerMessage::StopRecording { abort } => {
                    if let Some(active) = state.recording_active.take() {
                        active.store(false, Ordering::Release);
                    }
                    if abort {
                        if let Some(aborted) = state.recording_aborted.take() {
                            aborted.store(true, Ordering::Release);
                        }
                    }
                    state.recording_stream = None;
                    state.recording_buffer = None;
                }
            },
            ClientLoopEvent::ServerDisconnected => {
                return Err(ClientError::ConnectionLost(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "server closed connection",
                )));
            }
            ClientLoopEvent::Timer => {
                // Check if we should quit.
            }
            ClientLoopEvent::RecordingStarted(stream) => {
                state.recording_stream = Some(stream);
            }
            ClientLoopEvent::ClientMessageToSend(msg) => {
                if let Err(e) = write_to_server(&mut write_stream, &msg) {
                    return Err(ClientError::ConnectionLost(e));
                }
            }
        }
    }

    // Clean exit (Ctrl+C). Send Detach before closing.
    let detach = ClientMessage::Detach;
    let _ = write_to_server(&mut write_stream, &detach);
    let _ = io::stdout().flush();

    Ok(())
}

// ---------------------------------------------------------------------------
// Server reader thread
// ---------------------------------------------------------------------------

/// Blocking thread that reads ServerMessages from the server and sends them
/// to the main event loop.
fn server_reader_thread(
    mut stream: UnixStream,
    event_tx: tokio::sync::mpsc::Sender<ClientLoopEvent>,
    should_quit: &Arc<AtomicBool>,
    max_frame_size: usize,
) {
    // Ensure the read stream is in blocking mode to avoid WouldBlock errors
    // from read_exact inside read_message. The stream should already be
    // blocking after handshake, but we enforce it here as a safety measure.
    if stream.set_nonblocking(false).is_err() {
        // If we can't set blocking mode, the stream is likely broken.
        let _ = event_tx.blocking_send(ClientLoopEvent::ServerDisconnected);
        return;
    }

    loop {
        if should_quit.load(Ordering::Acquire) {
            break;
        }

        match protocol::read_message(&mut stream, max_frame_size) {
            Ok(msg) => {
                if event_tx
                    .blocking_send(ClientLoopEvent::ServerMessage(msg))
                    .is_err()
                {
                    break; // Main loop gone.
                }
            }
            Err(protocol::FramingError::UnexpectedEof) => {
                // Server closed connection.
                let _ = event_tx.blocking_send(ClientLoopEvent::ServerDisconnected);
                break;
            }
            Err(protocol::FramingError::Io(err)) if err.kind() == io::ErrorKind::WouldBlock => {
                // Should not happen with blocking mode, but handle gracefully
                // in case the stream was set nonblocking by another clone.
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            Err(err) => {
                warn!(err = %err, "server read error");
                let _ = event_tx.blocking_send(ClientLoopEvent::ServerDisconnected);
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Write helper
// ---------------------------------------------------------------------------

/// Writes a message to the server stream (blocking).
fn write_to_server(stream: &mut UnixStream, msg: &ClientMessage) -> io::Result<()> {
    protocol::write_message(stream, msg).map_err(|e| io::Error::other(e.to_string()))
}

// ---------------------------------------------------------------------------
// Notifications
// ---------------------------------------------------------------------------

fn reload_local_client_config(
    sound_config: &mut crate::config::SoundConfig,
    redraw_on_focus_gained: &mut bool,
) {
    match crate::config::load_live_config() {
        Ok(loaded) => {
            for diagnostic in loaded.config.ui.sound.diagnostics() {
                warn!(diagnostic = %diagnostic, "local sound config diagnostic");
            }
            *sound_config = loaded.config.ui.sound;
            *redraw_on_focus_gained = loaded.config.ui.redraw_on_focus_gained;
            debug!("reloaded local client config");
        }
        Err(diagnostics) => {
            warn!(diagnostics = ?diagnostics, "failed to reload local client config; keeping current client config");
        }
    }
}

fn handle_notify(kind: NotifyKind, message: &str, sound_config: &crate::config::SoundConfig) {
    handle_notify_with_notifiers(
        kind,
        message,
        sound_config,
        crate::terminal_notify::show_notification,
        crate::platform::show_desktop_notification,
    );
}

fn handle_notify_with_notifiers(
    kind: NotifyKind,
    message: &str,
    sound_config: &crate::config::SoundConfig,
    mut show_terminal_notification: impl FnMut(&str, Option<&str>) -> io::Result<bool>,
    mut show_system_notification: impl FnMut(&str, Option<&str>) -> io::Result<bool>,
) {
    match kind {
        NotifyKind::Sound => {
            let Some(sound) = sound_from_notify_message(message) else {
                warn!(
                    message = message,
                    "received unknown sound notification from server"
                );
                return;
            };
            if sound_config.enabled {
                crate::sound::play(sound, sound_config);
            }
        }
        NotifyKind::Toast => {
            debug!(
                message = message,
                "received terminal toast notification from server"
            );
            let (title, body) = crate::terminal_notify::split_message(message);
            if let Err(err) = show_terminal_notification(title, body) {
                warn!(err = %err, "failed to emit terminal notification");
            }
        }
        NotifyKind::SystemToast => {
            debug!(
                message = message,
                "received system toast notification from server"
            );
            let (title, body) = crate::terminal_notify::split_message(message);
            if let Err(err) = show_system_notification(title, body) {
                warn!(err = %err, "failed to emit system notification");
            }
        }
    }
}

fn sound_from_notify_message(message: &str) -> Option<crate::sound::Sound> {
    match message {
        "agent done" => Some(crate::sound::Sound::Done),
        "agent attention" => Some(crate::sound::Sound::Request),
        _ => None,
    }
}

fn should_bridge_clipboard_image_paste(data: &[u8]) -> bool {
    if data == b"\x1b[200~\x1b[201~" {
        return true;
    }

    let events = crate::raw_input::parse_raw_input_bytes_sync(data);
    matches!(
        events.as_slice(),
        [crate::raw_input::RawInputEvent::Key(key)]
            if key.kind == crossterm::event::KeyEventKind::Press
                && key.modifiers == crossterm::event::KeyModifiers::CONTROL
                && matches!(key.code, crossterm::event::KeyCode::Char('v' | 'V'))
    )
}

// ---------------------------------------------------------------------------
// Clipboard forwarding
// ---------------------------------------------------------------------------

/// Decode a clipboard payload forwarded by the server.
fn decode_clipboard_payload(data: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(data).ok()
}

/// Forwards a clipboard write from the server to the local client clipboard.
fn forward_clipboard(data: &str) {
    let Some(bytes) = decode_clipboard_payload(data) else {
        warn!("received invalid clipboard payload from server");
        return;
    };

    crate::selection::write_osc52_bytes(&bytes);
}

// ---------------------------------------------------------------------------
// Frame output
// ---------------------------------------------------------------------------

fn write_encoded_frame_with_graphics(
    mut writer: impl io::Write,
    encoded: &[u8],
    graphics: &[u8],
) -> io::Result<()> {
    writer.write_all(encoded)?;
    if graphics.is_empty() {
        return Ok(());
    }

    record_received_kitty_graphics(graphics);
    writer.write_all(b"\x1b7")?;
    writer.write_all(graphics)?;
    writer.write_all(b"\x1b8")
}

fn contains_kitty_graphics_bytes(bytes: &[u8]) -> bool {
    bytes.windows(3).any(|window| window == b"\x1b_G")
}

fn record_received_kitty_graphics(bytes: &[u8]) {
    let ids = kitty_graphics_image_ids(bytes);
    if ids.is_empty() {
        return;
    }
    let set = RECEIVED_KITTY_GRAPHICS_IDS.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut set) = set.lock() {
        set.extend(ids);
    }
}

fn clear_received_kitty_graphics(mut writer: impl io::Write) -> io::Result<()> {
    let Some(set) = RECEIVED_KITTY_GRAPHICS_IDS.get() else {
        return Ok(());
    };
    let Ok(mut set) = set.lock() else {
        return Ok(());
    };
    for id in set.drain() {
        write!(writer, "\x1b_Ga=d,d=I,i={id},q=2;\x1b\\")?;
    }
    writer.flush()
}

fn kitty_graphics_image_ids(bytes: &[u8]) -> Vec<u32> {
    let mut ids = Vec::new();
    let mut index = 0usize;
    while let Some(start) = find_subslice(&bytes[index..], b"\x1b_G") {
        let command_start = index + start + 3;
        let Some(end) = find_subslice(&bytes[command_start..], b"\x1b\\") else {
            break;
        };
        let command = &bytes[command_start..command_start + end];
        if let Some(id) = kitty_graphics_command_image_id(command) {
            ids.push(id);
        }
        index = command_start + end + 2;
    }
    ids
}

fn kitty_graphics_command_image_id(command: &[u8]) -> Option<u32> {
    let header_end = command
        .iter()
        .position(|byte| *byte == b';')
        .unwrap_or(command.len());
    for part in command[..header_end].split(|byte| *byte == b',') {
        let Some(value) = part.strip_prefix(b"i=") else {
            continue;
        };
        let text = std::str::from_utf8(value).ok()?;
        if let Ok(id) = text.parse::<u32>() {
            return Some(id);
        }
    }
    None
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

// ---------------------------------------------------------------------------
// Resize polling
// ---------------------------------------------------------------------------

fn current_terminal_geometry(kitty_graphics_enabled: bool) -> (u16, u16, u32, u32) {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    if !kitty_graphics_enabled {
        return (cols, rows, 0, 0);
    }
    let Ok(size) = crossterm::terminal::window_size() else {
        return (cols, rows, 8, 16);
    };
    if size.columns == 0 || size.rows == 0 || size.width == 0 || size.height == 0 {
        return (cols, rows, 8, 16);
    }
    (
        cols,
        rows,
        (size.width as u32 / size.columns as u32).max(1),
        (size.height as u32 / size.rows as u32).max(1),
    )
}

/// Polls the terminal size and sends resize events when it changes.
fn resize_poll_loop(
    resize_tx: tokio::sync::mpsc::Sender<ClientLoopEvent>,
    initial_cols: u16,
    initial_rows: u16,
    kitty_graphics_enabled: bool,
    should_quit: &Arc<AtomicBool>,
) {
    let (_, _, initial_cell_width, initial_cell_height) =
        current_terminal_geometry(kitty_graphics_enabled);
    let mut last_size = (
        initial_cols,
        initial_rows,
        initial_cell_width,
        initial_cell_height,
    );
    while !should_quit.load(Ordering::Acquire) {
        std::thread::sleep(Duration::from_millis(100));
        let new_size = current_terminal_geometry(kitty_graphics_enabled);
        if new_size != last_size {
            last_size = new_size;
            if resize_tx
                .blocking_send(ClientLoopEvent::Resize(
                    new_size.0, new_size.1, new_size.2, new_size.3,
                ))
                .is_err()
            {
                break; // Main loop gone.
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

/// Initialize logging for the client process.
fn query_host_terminal_theme() {
    let _ = write_host_terminal_theme_query(io::stdout());
}

fn write_host_terminal_theme_query(mut writer: impl io::Write) -> io::Result<()> {
    writer.write_all(crate::terminal_theme::HOST_COLOR_QUERY_SEQUENCE.as_bytes())?;
    writer.flush()
}

fn init_logging() {
    crate::logging::init_file_logging("herdr-client.log");
}

async fn run_client_websocket_transcription(
    workspace_id: String,
    pane_id: Option<crate::layout::PaneId>,
    api_key: String,
    model: String,
    system_instruction: String,
    buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    channels: u16,
    sample_rate: u32,
    recording_active: Arc<std::sync::atomic::AtomicBool>,
    recording_aborted: Arc<std::sync::atomic::AtomicBool>,
    event_tx: tokio::sync::mpsc::Sender<ClientLoopEvent>,
) {
    use base64::Engine;
    use futures_util::{SinkExt, StreamExt};

    let model_name = if model.starts_with("models/") {
        model
    } else {
        format!("models/{}", model)
    };

    let url = format!(
        "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent?key={}",
        api_key
    );

    tracing::info!("Speech to Text client: connecting to Gemini Live WebSocket...");
    let ws_stream = match tokio_tungstenite::connect_async(&url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            tracing::error!(
                "Speech to Text client: failed to connect to Gemini Live WebSocket: {}",
                e
            );
            let _ = event_tx
                .send(ClientLoopEvent::ClientMessageToSend(
                    ClientMessage::SpeechTranscribed {
                        workspace_id,
                        pane_id,
                        result: Err(format!("WebSocket connection failed: {}", e)),
                    },
                ))
                .await;
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    let setup_msg = serde_json::json!({
        "setup": {
            "model": model_name,
            "generationConfig": {
                "responseModalities": ["AUDIO"],
                "maxOutputTokens": 1
            },
            "systemInstruction": {
                "parts": [
                    {
                        "text": system_instruction
                    }
                ]
            },
            "inputAudioTranscription": {},
            "outputAudioTranscription": {}
        }
    });

    tracing::info!("Speech to Text client: sending setup message...");
    if let Err(e) = write
        .send(tokio_tungstenite::tungstenite::Message::Text(
            setup_msg.to_string(),
        ))
        .await
    {
        tracing::error!("Speech to Text client: failed to send setup message: {}", e);
        let _ = event_tx
            .send(ClientLoopEvent::ClientMessageToSend(
                ClientMessage::SpeechTranscribed {
                    workspace_id,
                    pane_id,
                    result: Err(format!("Failed to send setup message: {}", e)),
                },
            ))
            .await;
        return;
    }

    let mut finalized_text = String::new();
    let mut current_turn_text = String::new();
    let mut recording_stopped = false;
    let mut stop_time: Option<std::time::Instant> = None;

    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

    loop {
        if recording_aborted.load(std::sync::atomic::Ordering::Acquire) {
            tracing::info!("Speech to Text client: transcription aborted");
            return;
        }

        tokio::select! {
            _ = interval.tick() => {
                if !recording_stopped {
                    if !recording_active.load(std::sync::atomic::Ordering::Acquire) {
                        tracing::info!("Speech to Text client: stop recording detected, sending turnComplete...");
                        recording_stopped = true;
                        stop_time = Some(std::time::Instant::now());

                        let turn_msg = serde_json::json!({
                            "realtimeInput": {
                                "audioStreamEnd": true
                            }
                        });

                        if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Text(turn_msg.to_string())).await {
                            tracing::error!("Speech to Text client: failed to send audioStreamEnd message: {}", e);
                        }
                        continue;
                    }

                    let new_samples = {
                        match buffer.lock() {
                            Ok(mut buf) => std::mem::take(&mut *buf),
                            Err(_) => Vec::new(),
                        }
                    };

                    if !new_samples.is_empty() {
                        let mono = crate::app::speech::downmix_to_mono(&new_samples, channels);
                        let resampled = crate::app::speech::resample(&mono, sample_rate, 16000);

                        let mut raw_bytes = Vec::with_capacity(resampled.len() * 2);
                        for &sample in &resampled {
                            let clamped = sample.clamp(-1.0, 1.0);
                            let scaled = (clamped * 32767.0) as i16;
                            raw_bytes.extend_from_slice(&scaled.to_le_bytes());
                        }

                        let base64_audio = base64::prelude::BASE64_STANDARD.encode(&raw_bytes);

                        let media_msg = serde_json::json!({
                            "realtimeInput": {
                                "audio": {
                                    "mimeType": "audio/pcm;rate=16000",
                                    "data": base64_audio
                                }
                            }
                        });

                        if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Text(media_msg.to_string())).await {
                            tracing::error!("Speech to Text client: failed to send audio chunk: {}", e);
                            break;
                        }
                    }
                }
            }

            msg_res = read.next() => {
                let msg = match msg_res {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        tracing::error!("Speech to Text client: WebSocket read error: {}", e);
                        break;
                    }
                    None => {
                        tracing::info!("Speech to Text client: WebSocket stream ended by server");
                        break;
                    }
                };

                let text_opt = match &msg {
                    tokio_tungstenite::tungstenite::Message::Text(text) => Some(text.clone()),
                    tokio_tungstenite::tungstenite::Message::Binary(bin) => String::from_utf8(bin.clone()).ok(),
                    _ => None,
                };

                if let Some(text) = text_opt {
                    if let Some((partial, turn_complete)) = crate::app::speech::parse_live_transcription_frame(&text) {
                        if !partial.is_empty() {
                            current_turn_text.push_str(&partial);

                            let mut full_text = finalized_text.clone();
                            if !full_text.is_empty() && !current_turn_text.is_empty() {
                                full_text.push(' ');
                            }
                            full_text.push_str(&current_turn_text);

                            let _ = event_tx.send(ClientLoopEvent::ClientMessageToSend(
                                ClientMessage::SpeechPartialTranscription {
                                    workspace_id: workspace_id.clone(),
                                    text: full_text,
                                }
                            )).await;
                        }
                        if turn_complete {
                            tracing::info!("Speech to Text client: turnComplete received from server");
                            if !current_turn_text.is_empty() {
                                if !finalized_text.is_empty() {
                                    finalized_text.push(' ');
                                }
                                finalized_text.push_str(&current_turn_text);
                                current_turn_text = String::new();
                            }
                            if recording_stopped {
                                break;
                            }
                        }
                    }
                }

                if let tokio_tungstenite::tungstenite::Message::Close(frame) = msg {
                    tracing::info!("Speech to Text client: Close frame received: {:?}", frame);
                    break;
                }
            }
        }

        if recording_stopped {
            if let Some(t) = stop_time {
                if t.elapsed() > tokio::time::Duration::from_secs(5) {
                    tracing::warn!("Speech to Text client: grace period of 5 seconds expired waiting for final transcription");
                    break;
                }
            }
        }
    }

    if recording_aborted.load(std::sync::atomic::Ordering::Acquire) {
        return;
    }

    let mut final_text = finalized_text.clone();
    if !current_turn_text.is_empty() {
        if !final_text.is_empty() {
            final_text.push(' ');
        }
        final_text.push_str(&current_turn_text);
    }

    let raw_result = if !final_text.trim().is_empty() {
        Ok(final_text.trim().to_string())
    } else {
        Err("No speech detected / transcription empty.".to_string())
    };

    let post_result = match raw_result {
        Ok(text) => {
            let api_key_clone = api_key.clone();
            let system_instruction = "You are a transcription formatting assistant. Fix punctuation, capitalization, grammatical spacing, and spelling errors in the provided raw transcription. Preserve the exact content, words, and semantic meaning. Output only the corrected text.".to_string();
            tokio::task::spawn_blocking(move || {
                crate::app::speech::run_gemini_postprocess(api_key_clone, text, system_instruction)
            })
            .await
            .unwrap_or_else(|e| Err(format!("Post-process task join error: {}", e)))
        }
        Err(e) => Err(e),
    };

    let _ = event_tx
        .send(ClientLoopEvent::ClientMessageToSend(
            ClientMessage::SpeechTranscribed {
                workspace_id,
                pane_id,
                result: post_result,
            },
        ))
        .await;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn restore_env_var(key: &str, value: Option<OsString>) {
        if let Some(value) = value {
            std::env::set_var(key, value);
        } else {
            std::env::remove_var(key);
        }
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            restore_env_var(self.key, self.previous.clone());
        }
    }

    struct EnvVarsRemovedGuard {
        previous: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvVarsRemovedGuard {
        fn new(keys: &[&'static str]) -> Self {
            let previous: Vec<_> = keys
                .iter()
                .map(|key| (*key, std::env::var_os(key)))
                .collect();
            for key in keys {
                std::env::remove_var(key);
            }
            Self { previous }
        }
    }

    impl Drop for EnvVarsRemovedGuard {
        fn drop(&mut self) {
            for (key, value) in self.previous.clone() {
                restore_env_var(key, value);
            }
        }
    }

    #[test]
    fn clipboard_image_paste_bridge_triggers_on_ctrl_v_and_empty_paste() {
        assert!(should_bridge_clipboard_image_paste(&[0x16]));
        assert!(should_bridge_clipboard_image_paste(b"\x1b[118;5u"));
        assert!(should_bridge_clipboard_image_paste(b"\x1b[200~\x1b[201~"));
        assert!(!should_bridge_clipboard_image_paste(
            b"\x1b[200~text\x1b[201~"
        ));
        assert!(!should_bridge_clipboard_image_paste(b"v"));
    }

    #[test]
    fn graphics_bytes_are_written_after_blit_with_saved_cursor() {
        let mut output = Vec::new();
        write_encoded_frame_with_graphics(
            &mut output,
            b"\x1b[?2026htext\x1b[?2026lcursor",
            b"graphics",
        )
        .unwrap();

        assert_eq!(
            output,
            b"\x1b[?2026htext\x1b[?2026lcursor\x1b7graphics\x1b8"
        );
    }

    #[test]
    fn empty_graphics_writes_only_blit_frame() {
        let mut output = Vec::new();
        write_encoded_frame_with_graphics(&mut output, b"text", b"").unwrap();

        assert_eq!(output, b"text");
    }

    #[test]
    fn terminal_frame_kitty_detection_matches_apc_prefix() {
        assert!(contains_kitty_graphics_bytes(b"text\x1b_Ga=p;\x1b\\"));
        assert!(!contains_kitty_graphics_bytes(b"text\x1b[?2026h"));
    }

    #[test]
    fn kitty_graphics_image_id_parser_tracks_herdr_ids_only() {
        let ids = kitty_graphics_image_ids(
            b"text\x1b_Ga=t,t=d,f=32,s=1,v=1,i=10023,q=2;AAAA\x1b\\\x1b_Ga=p,i=10023,p=7;\x1b\\",
        );
        assert_eq!(ids, vec![10023, 10023]);
    }

    #[test]
    fn kitty_graphics_cleanup_deletes_tracked_images_not_all_images() {
        record_received_kitty_graphics(b"\x1b_Ga=t,i=123,q=2;AAAA\x1b\\");
        let mut output = Vec::new();
        clear_received_kitty_graphics(&mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("a=d,d=I,i=123"));
        assert!(!text.contains("d=A"));
    }

    #[test]
    fn write_host_terminal_theme_query_emits_osc_queries() {
        let mut output = Vec::new();
        write_host_terminal_theme_query(&mut output).unwrap();
        assert_eq!(
            output,
            crate::terminal_theme::HOST_COLOR_QUERY_SEQUENCE.as_bytes()
        );
    }

    #[test]
    fn terminal_restore_postlude_restores_visible_default_cursor() {
        let mut output = Vec::new();
        write_terminal_restore_postlude(&mut output).unwrap();
        assert_eq!(output, b"\x1b[?25h\x1b[0 q");
    }

    #[test]
    fn attach_escape_detaches_on_prefix_q() {
        let mut escape = AttachEscapeState::default();
        assert!(matches!(
            escape.filter_input(vec![0x02], 24, 3),
            AttachInputAction::None
        ));
        assert!(matches!(
            escape.filter_input(vec![b'q'], 24, 3),
            AttachInputAction::Detach
        ));
    }

    #[test]
    fn attach_escape_sends_literal_prefix_on_double_prefix() {
        let mut escape = AttachEscapeState::default();
        assert!(matches!(
            escape.filter_input(vec![0x02], 24, 3),
            AttachInputAction::None
        ));
        match escape.filter_input(vec![0x02], 24, 3) {
            AttachInputAction::Forward(bytes) => assert_eq!(bytes, vec![0x02]),
            other => panic!("expected forwarded prefix, got {other:?}"),
        }
    }

    #[test]
    fn attach_escape_forwards_prefix_before_non_escape_key() {
        let mut escape = AttachEscapeState::default();
        assert!(matches!(
            escape.filter_input(vec![b'a', 0x02], 24, 3),
            AttachInputAction::Forward(bytes) if bytes == b"a"
        ));
        match escape.filter_input(vec![b'x'], 24, 3) {
            AttachInputAction::Forward(bytes) => assert_eq!(bytes, vec![0x02, b'x']),
            other => panic!("expected forwarded bytes, got {other:?}"),
        }
    }

    #[test]
    fn attach_escape_turns_wheel_into_scroll_action() {
        let mut escape = AttachEscapeState::default();
        match escape.filter_input(b"\x1b[<64;11;6M".to_vec(), 24, 7) {
            AttachInputAction::Scroll {
                source,
                direction,
                lines,
                column,
                row,
                ..
            } => {
                assert_eq!(source, AttachScrollSource::Wheel);
                assert_eq!(direction, AttachScrollDirection::Up);
                assert_eq!(lines, 7);
                assert_eq!(column, Some(10));
                assert_eq!(row, Some(5));
            }
            other => panic!("expected scroll action, got {other:?}"),
        }
    }

    #[test]
    fn attach_escape_swallows_non_wheel_mouse_reports() {
        let mut escape = AttachEscapeState::default();
        assert!(matches!(
            escape.filter_input(b"\x1b[<0;11;6M".to_vec(), 24, 7),
            AttachInputAction::None
        ));
    }

    #[test]
    fn attach_escape_turns_plain_page_keys_into_scroll_actions() {
        let mut escape = AttachEscapeState::default();
        match escape.filter_input(b"\x1b[5~".to_vec(), 12, 3) {
            AttachInputAction::Scroll {
                source,
                direction,
                lines,
                ..
            } => {
                assert_eq!(
                    source,
                    AttachScrollSource::PageKey {
                        input: b"\x1b[5~".to_vec()
                    }
                );
                assert_eq!(direction, AttachScrollDirection::Up);
                assert_eq!(lines, 11);
            }
            other => panic!("expected page-up scroll action, got {other:?}"),
        }

        match escape.filter_input(b"\x1b[6~".to_vec(), 12, 3) {
            AttachInputAction::Scroll {
                source,
                direction,
                lines,
                ..
            } => {
                assert_eq!(
                    source,
                    AttachScrollSource::PageKey {
                        input: b"\x1b[6~".to_vec()
                    }
                );
                assert_eq!(direction, AttachScrollDirection::Down);
                assert_eq!(lines, 11);
            }
            other => panic!("expected page-down scroll action, got {other:?}"),
        }
    }

    #[test]
    fn attach_escape_forwards_modified_page_key() {
        let mut escape = AttachEscapeState::default();
        match escape.filter_input(b"\x1b[5;5~".to_vec(), 12, 3) {
            AttachInputAction::Forward(bytes) => assert_eq!(bytes, b"\x1b[5;5~"),
            other => panic!("expected modified page key to forward, got {other:?}"),
        }
    }

    #[test]
    fn client_error_display_connection_failed() {
        let err = ClientError::ConnectionFailed(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "connection refused",
        ));
        let msg = err.to_string();
        assert!(
            msg.contains("failed to connect to server"),
            "should mention connection failure: {msg}"
        );
        assert!(
            msg.contains("herdr server"),
            "should suggest starting server: {msg}"
        );
    }

    #[test]
    fn client_error_display_handshake_rejected() {
        let err = ClientError::HandshakeRejected {
            version: 1,
            error: "incompatible".into(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("rejected handshake"),
            "should mention rejection: {msg}"
        );
        assert!(msg.contains("incompatible"), "should include error: {msg}");
    }

    #[test]
    fn client_error_display_server_shutdown() {
        let err = ClientError::ServerShutdown {
            reason: Some("maintenance".into()),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("server shut down"),
            "should mention shutdown: {msg}"
        );
        assert!(msg.contains("maintenance"), "should include reason: {msg}");
    }

    #[test]
    fn client_error_display_server_shutdown_no_reason() {
        let err = ClientError::ServerShutdown { reason: None };
        let msg = err.to_string();
        assert!(
            msg.contains("server shut down"),
            "should mention shutdown: {msg}"
        );
    }

    #[test]
    fn client_error_display_detached_default_session_reattach_hint() {
        let _guard = env_lock().lock().unwrap();
        let _env = EnvVarsRemovedGuard::new(&[
            crate::remote::REATTACH_COMMAND_ENV_VAR,
            crate::session::SESSION_ENV_VAR,
        ]);
        let err = ClientError::ServerShutdown {
            reason: Some("detached".into()),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("Run `herdr` to reattach"),
            "should suggest default reattach command: {msg}"
        );
    }

    #[test]
    fn client_error_display_detached_named_session_reattach_hint() {
        let _guard = env_lock().lock().unwrap();
        let _remote_env = EnvVarsRemovedGuard::new(&[crate::remote::REATTACH_COMMAND_ENV_VAR]);
        let _session_env = EnvVarGuard::set(crate::session::SESSION_ENV_VAR, "work");
        let err = ClientError::ServerShutdown {
            reason: Some("detached".into()),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("Run `herdr session attach work` to reattach"),
            "should suggest named session reattach command: {msg}"
        );
    }

    #[test]
    fn client_error_display_detached_remote_reattach_hint_takes_precedence() {
        let _guard = env_lock().lock().unwrap();
        let _remote_env = EnvVarGuard::set(
            crate::remote::REATTACH_COMMAND_ENV_VAR,
            "herdr --remote host --session work",
        );
        let _session_env = EnvVarGuard::set(crate::session::SESSION_ENV_VAR, "work");
        let err = ClientError::ServerShutdown {
            reason: Some("detached".into()),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("Run `herdr --remote host --session work` to reattach"),
            "should prefer remote reattach command: {msg}"
        );
    }

    #[test]
    fn client_error_display_connection_lost() {
        let err =
            ClientError::ConnectionLost(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"));
        let msg = err.to_string();
        assert!(
            msg.contains("lost connection to server"),
            "should mention lost connection: {msg}"
        );
    }

    #[test]
    fn sound_from_notify_message_maps_done() {
        assert_eq!(
            sound_from_notify_message("agent done"),
            Some(crate::sound::Sound::Done)
        );
    }

    #[test]
    fn sound_from_notify_message_maps_attention() {
        assert_eq!(
            sound_from_notify_message("agent attention"),
            Some(crate::sound::Sound::Request)
        );
    }

    #[test]
    fn sound_from_notify_message_rejects_unknown_payloads() {
        assert_eq!(sound_from_notify_message("toast"), None);
    }

    #[test]
    fn reload_local_client_config_refreshes_redraw_on_focus_gained() {
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        let path = std::env::temp_dir().join(format!(
            "herdr-client-config-reload-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, "[ui]\nredraw_on_focus_gained = false\n").unwrap();
        let path_string = path.to_string_lossy().to_string();
        let _env = EnvVarGuard::set(crate::config::CONFIG_PATH_ENV_VAR, &path_string);
        let mut sound_config = crate::config::SoundConfig::default();
        let mut redraw_on_focus_gained = true;

        reload_local_client_config(&mut sound_config, &mut redraw_on_focus_gained);

        assert!(!redraw_on_focus_gained);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn toast_notify_from_server_is_emitted_even_when_attach_config_was_off() {
        let sound_config = crate::config::SoundConfig::default();
        let mut emitted = None;

        handle_notify_with_notifiers(
            NotifyKind::Toast,
            "pi finished: workspace 1",
            &sound_config,
            |title, body| {
                emitted = Some((title.to_string(), body.map(str::to_string)));
                Ok(true)
            },
            |_, _| Ok(false),
        );

        assert_eq!(
            emitted,
            Some(("pi finished".to_string(), Some("workspace 1".to_string())))
        );
    }

    #[test]
    fn system_toast_notify_from_server_uses_system_notifier() {
        let sound_config = crate::config::SoundConfig::default();
        let mut emitted = None;

        handle_notify_with_notifiers(
            NotifyKind::SystemToast,
            "pi finished: workspace 1",
            &sound_config,
            |_, _| Ok(false),
            |title, body| {
                emitted = Some((title.to_string(), body.map(str::to_string)));
                Ok(true)
            },
        );

        assert_eq!(
            emitted,
            Some(("pi finished".to_string(), Some("workspace 1".to_string())))
        );
    }

    #[test]
    fn decode_clipboard_payload_decodes_base64() {
        assert_eq!(decode_clipboard_payload("dGVzdA=="), Some(b"test".to_vec()));
    }

    #[test]
    fn decode_clipboard_payload_rejects_invalid_base64() {
        assert_eq!(decode_clipboard_payload("not-base64!!!"), None);
    }

    #[test]
    fn forward_clipboard_uses_local_clipboard_path() {
        unsafe {
            std::env::set_var("SSH_CONNECTION", "1 2 3 4");
        }
        forward_clipboard("dGVzdA==");
        unsafe {
            std::env::remove_var("SSH_CONNECTION");
        }
    }
}
