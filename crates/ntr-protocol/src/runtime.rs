use crate::classic::{ClassicAssembler, ClassicPacket, PacketOutcome};
use crate::control::ControlHeader;
use crate::{ConnectionState, Frame, StreamMode, ViewerCommand, ViewerConfig, ViewerEvent};
use crossbeam_channel::{Receiver, Sender, TryRecvError, bounded};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const CONTROL_PORT: u16 = 8000;
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(250);
const FALLBACK_INTERVAL: Duration = Duration::from_secs(3);

pub struct Viewer;

pub struct ViewerHandle {
    commands: Sender<ViewerCommand>,
    events: Receiver<ViewerEvent>,
    thread: Option<JoinHandle<()>>,
}

impl Viewer {
    pub fn spawn() -> ViewerHandle {
        let (command_tx, command_rx) = bounded(8);
        let (event_tx, event_rx) = bounded(8);
        let thread = thread::Builder::new()
            .name("ntr-viewer".into())
            .spawn(move || worker(command_rx, event_tx))
            .expect("failed to spawn viewer thread");
        ViewerHandle {
            commands: command_tx,
            events: event_rx,
            thread: Some(thread),
        }
    }
}

impl ViewerHandle {
    pub fn send(
        &self,
        command: ViewerCommand,
    ) -> Result<(), crossbeam_channel::SendError<ViewerCommand>> {
        self.commands.send(command)
    }

    pub fn try_event(&self) -> Result<ViewerEvent, TryRecvError> {
        self.events.try_recv()
    }
}

impl Drop for ViewerHandle {
    fn drop(&mut self) {
        let _ = self.commands.send(ViewerCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn worker(commands: Receiver<ViewerCommand>, events: Sender<ViewerEvent>) {
    while let Ok(command) = commands.recv() {
        match command {
            ViewerCommand::Connect(config) => {
                let _ = events.send(ViewerEvent::StateChanged(ConnectionState::Connecting));
                if let Err(error) = run_session(config, &commands, &events) {
                    let _ = events.send(ViewerEvent::Error(error));
                }
                let _ = events.send(ViewerEvent::StateChanged(ConnectionState::Disconnected));
            }
            ViewerCommand::Shutdown => break,
            ViewerCommand::Disconnect => {}
        }
    }
}

fn run_session(
    config: ViewerConfig,
    commands: &Receiver<ViewerCommand>,
    events: &Sender<ViewerEvent>,
) -> Result<(), String> {
    let udp_addr = SocketAddr::new(config.bind_ip, config.viewer_port);
    let udp = UdpSocket::bind(udp_addr).map_err(|e| format!("could not bind {udp_addr}: {e}"))?;
    udp.set_read_timeout(Some(Duration::from_millis(40)))
        .map_err(|e| e.to_string())?;

    let control_addr = SocketAddr::new(config.console_ip, CONTROL_PORT);
    let mut tcp = TcpStream::connect_timeout(&control_addr, Duration::from_secs(2))
        .map_err(|e| format!("could not connect to NTR at {control_addr}: {e}"))?;
    tcp.set_nonblocking(true).map_err(|e| e.to_string())?;
    tcp.set_nodelay(true).map_err(|e| e.to_string())?;

    let auto_modes = [
        StreamMode::JpegReliableDelta,
        StreamMode::JpegReliable,
        StreamMode::JpegCompat,
    ];
    let explicit_mode = config.stream_mode.command_mode();
    let mut auto_index = 0;
    let mut active_mode = if config.stream_mode == StreamMode::Auto {
        auto_modes[0]
    } else {
        explicit_mode
    };
    let mut sequence = 0_u32;
    send_header(&mut tcp, ControlHeader::heartbeat(sequence))?;
    sequence += 1;
    send_header(
        &mut tcp,
        ControlHeader::remote_play(sequence, &config, active_mode),
    )?;
    sequence += 1;
    let _ = events.send(ViewerEvent::ActiveMode(active_mode));

    let mut assembler = ClassicAssembler::default();
    let mut datagram = vec![0_u8; 64 * 1024];
    let mut tcp_discard = [0_u8; 4096];
    let mut last_heartbeat = Instant::now();
    let mut mode_started = Instant::now();
    let mut last_stats = Instant::now();
    let mut decoded = 0_u64;
    let mut dropped = 0_u64;
    let mut streaming = false;

    loop {
        match commands.try_recv() {
            Ok(ViewerCommand::Disconnect) | Ok(ViewerCommand::Shutdown) => return Ok(()),
            Ok(ViewerCommand::Connect(_)) => return Ok(()),
            Err(TryRecvError::Disconnected) => return Ok(()),
            Err(TryRecvError::Empty) => {}
        }

        if last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
            send_header(&mut tcp, ControlHeader::heartbeat(sequence))?;
            sequence = sequence.wrapping_add(1);
            last_heartbeat = Instant::now();
        }
        loop {
            match tcp.read(&mut tcp_discard) {
                Ok(0) => return Err("NTR closed the control connection".into()),
                Ok(_) => continue,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(format!("NTR control read failed: {e}")),
            }
        }

        if !streaming
            && config.stream_mode == StreamMode::Auto
            && mode_started.elapsed() >= FALLBACK_INTERVAL
            && auto_index + 1 < auto_modes.len()
        {
            auto_index += 1;
            active_mode = auto_modes[auto_index];
            assembler = ClassicAssembler::default();
            send_header(
                &mut tcp,
                ControlHeader::remote_play(sequence, &config, active_mode),
            )?;
            sequence = sequence.wrapping_add(1);
            mode_started = Instant::now();
            let _ = events.send(ViewerEvent::ActiveMode(active_mode));
        }

        match udp.recv_from(&mut datagram) {
            Ok((len, remote)) => {
                if remote.ip() != config.console_ip || active_mode != StreamMode::JpegCompat {
                    continue;
                }
                match ClassicPacket::parse(&datagram[..len]) {
                    Ok(packet) => match assembler.push(packet) {
                        PacketOutcome::Complete(frame) if !frame.lossless => {
                            match ntr_codec::decode_jpeg(&frame.encoded) {
                                Ok(image) => {
                                    decoded += 1;
                                    if !streaming {
                                        streaming = true;
                                        let _ = events.send(ViewerEvent::StateChanged(
                                            ConnectionState::Streaming,
                                        ));
                                    }
                                    let event = ViewerEvent::Frame(Frame {
                                        screen: frame.screen,
                                        sequence: frame.sequence,
                                        width: image.width,
                                        height: image.height,
                                        rgba: Arc::from(image.rgba),
                                    });
                                    if events.try_send(event).is_err() {
                                        dropped += 1;
                                    }
                                }
                                Err(error) => {
                                    dropped += 1;
                                    let _ = events.try_send(ViewerEvent::Error(error.to_string()));
                                }
                            }
                        }
                        PacketOutcome::Complete(_) | PacketOutcome::ReplacedIncomplete => {
                            dropped += 1
                        }
                        PacketOutcome::Pending => {}
                    },
                    Err(_) => dropped += 1,
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(format!("UDP receive failed: {e}")),
        }

        if last_stats.elapsed() >= Duration::from_secs(1) {
            let _ = events.try_send(ViewerEvent::Stats { decoded, dropped });
            last_stats = Instant::now();
        }
    }
}

fn send_header(tcp: &mut TcpStream, header: ControlHeader) -> Result<(), String> {
    let bytes = header.encode();
    let mut sent = 0;
    while sent < bytes.len() {
        match tcp.write(&bytes[sent..]) {
            Ok(0) => return Err("NTR closed the control connection".into()),
            Ok(count) => sent += count,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(e) => return Err(format!("NTR control write failed: {e}")),
        }
    }
    Ok(())
}
