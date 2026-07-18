use eframe::egui;
use ntr_protocol::{
    ConnectionState, Frame, Screen, StreamMode, Viewer, ViewerCommand, ViewerConfig, ViewerEvent,
    ViewerHandle,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RustyNTRViewer")
            .with_inner_size([900.0, 760.0])
            .with_min_inner_size([520.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native(
        "RustyNTRViewer",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
enum Layout {
    #[default]
    Stacked,
    SideBySide,
    TopOnly,
    BottomOnly,
    Separate,
}

impl Layout {
    const ALL: [Self; 5] = [
        Self::Stacked,
        Self::SideBySide,
        Self::TopOnly,
        Self::BottomOnly,
        Self::Separate,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Stacked => "Stacked",
            Self::SideBySide => "Side by side",
            Self::TopOnly => "Top only",
            Self::BottomOnly => "Bottom only",
            Self::Separate => "Separate windows",
        }
    }
}

struct App {
    viewer: ViewerHandle,
    console_ip: String,
    bind_ip: String,
    viewer_port: u16,
    mode: StreamMode,
    layout: Layout,
    quality: u8,
    bandwidth: u8,
    priority_factor: u8,
    state: ConnectionState,
    active_mode: Option<StreamMode>,
    error: Option<String>,
    decoded: u64,
    dropped: u64,
    top: Option<egui::TextureHandle>,
    bottom: Option<egui::TextureHandle>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let stored = |key: &str, fallback: &str| {
            cc.storage
                .and_then(|storage| storage.get_string(key))
                .unwrap_or_else(|| fallback.to_owned())
        };
        let mode_index = stored("mode", "0").parse::<usize>().unwrap_or(0);
        let layout_index = stored("layout", "0").parse::<usize>().unwrap_or(0);
        Self {
            viewer: Viewer::spawn(),
            console_ip: stored("console_ip", "192.168.1.24"),
            bind_ip: stored("bind_ip", "192.168.1.5"),
            viewer_port: stored("viewer_port", "8001").parse().unwrap_or(8001),
            mode: StreamMode::ALL.get(mode_index).copied().unwrap_or_default(),
            layout: Layout::ALL.get(layout_index).copied().unwrap_or_default(),
            quality: 75,
            bandwidth: 16,
            priority_factor: 2,
            state: ConnectionState::Disconnected,
            active_mode: None,
            error: None,
            decoded: 0,
            dropped: 0,
            top: None,
            bottom: None,
        }
    }

    fn connect(&mut self) {
        let console_ip = match self.console_ip.parse::<IpAddr>() {
            Ok(ip) => ip,
            Err(error) => {
                self.error = Some(format!("Invalid 3DS IP: {error}"));
                return;
            }
        };
        let bind_ip = match self.bind_ip.parse::<IpAddr>() {
            Ok(ip) => ip,
            Err(error) => {
                self.error = Some(format!("Invalid Viewer IP: {error}"));
                return;
            }
        };
        self.error = None;
        let config = ViewerConfig {
            console_ip,
            bind_ip,
            viewer_port: self.viewer_port,
            stream_mode: self.mode,
            jpeg_quality: self.quality,
            bandwidth_mbps: self.bandwidth,
            top_screen_priority: true,
            priority_factor: self.priority_factor,
            lossless_color_bias: -1,
        };
        if let Err(error) = self.viewer.send(ViewerCommand::Connect(config)) {
            self.error = Some(error.to_string());
        }
    }

    fn receive_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.viewer.try_event() {
            match event {
                ViewerEvent::StateChanged(state) => self.state = state,
                ViewerEvent::ActiveMode(mode) => self.active_mode = Some(mode),
                ViewerEvent::Frame(frame) => self.update_texture(ctx, frame),
                ViewerEvent::Stats { decoded, dropped } => {
                    self.decoded = decoded;
                    self.dropped = dropped;
                }
                ViewerEvent::Error(error) => self.error = Some(error),
            }
            ctx.request_repaint();
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context, frame: Frame) {
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [frame.width as usize, frame.height as usize],
            &frame.rgba,
        );
        let target = match frame.screen {
            Screen::Top => &mut self.top,
            Screen::Bottom => &mut self.bottom,
        };
        match target {
            Some(texture) => texture.set(image, egui::TextureOptions::NEAREST),
            None => {
                *target = Some(ctx.load_texture(
                    match frame.screen {
                        Screen::Top => "top-screen",
                        Screen::Bottom => "bottom-screen",
                    },
                    image,
                    egui::TextureOptions::NEAREST,
                ))
            }
        }
    }

    fn screen(ui: &mut egui::Ui, texture: Option<&egui::TextureHandle>, max: egui::Vec2) {
        if let Some(texture) = texture {
            let size = texture.size_vec2();
            let scale = (max.x / size.x).min(max.y / size.y).max(0.1);
            ui.add(egui::Image::new(texture).fit_to_exact_size(size * scale));
        } else {
            ui.allocate_ui_with_layout(
                max,
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.weak("Waiting for video…");
                },
            );
        }
    }

    fn central_screens(&self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        match self.layout {
            Layout::Stacked => {
                Self::screen(
                    ui,
                    self.top.as_ref(),
                    egui::vec2(available.x, available.y * 0.52),
                );
                ui.separator();
                Self::screen(
                    ui,
                    self.bottom.as_ref(),
                    egui::vec2(available.x, available.y * 0.43),
                );
            }
            Layout::SideBySide => {
                ui.horizontal(|ui| {
                    Self::screen(
                        ui,
                        self.top.as_ref(),
                        egui::vec2(available.x * 0.55, available.y),
                    );
                    Self::screen(
                        ui,
                        self.bottom.as_ref(),
                        egui::vec2(available.x * 0.42, available.y),
                    );
                });
            }
            Layout::TopOnly => Self::screen(ui, self.top.as_ref(), available),
            Layout::BottomOnly => Self::screen(ui, self.bottom.as_ref(), available),
            Layout::Separate => {
                ui.centered_and_justified(|ui| ui.weak("Screens are open in separate windows."));
            }
        }
    }

    fn separate_windows(&self, ctx: &egui::Context) {
        if self.layout != Layout::Separate {
            return;
        }
        let top = self.top.clone();
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("top-screen-window"),
            egui::ViewportBuilder::default()
                .with_title("RustyNTRViewer — Top")
                .with_inner_size([800.0, 480.0]),
            move |ui, _| Self::screen(ui, top.as_ref(), ui.available_size()),
        );
        let bottom = self.bottom.clone();
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("bottom-screen-window"),
            egui::ViewportBuilder::default()
                .with_title("RustyNTRViewer — Bottom")
                .with_inner_size([640.0, 480.0]),
            move |ui, _| Self::screen(ui, bottom.as_ref(), ui.available_size()),
        );
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.receive_events(&ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
        self.separate_windows(&ctx);

        egui::Panel::top("controls").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("3DS IP");
                ui.add(egui::TextEdit::singleline(&mut self.console_ip).desired_width(120.0));
                ui.label("Viewer IP");
                ui.add(egui::TextEdit::singleline(&mut self.bind_ip).desired_width(120.0));
                ui.label("Port");
                ui.add(egui::DragValue::new(&mut self.viewer_port).range(1024..=65535));
                let label = if self.state == ConnectionState::Disconnected {
                    "Connect"
                } else {
                    "Disconnect"
                };
                if ui.button(label).clicked() {
                    if self.state == ConnectionState::Disconnected {
                        self.connect();
                    } else {
                        let _ = self.viewer.send(ViewerCommand::Disconnect);
                    }
                }
            });
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("mode")
                    .selected_text(self.mode.label())
                    .show_ui(ui, |ui| {
                        for mode in StreamMode::ALL {
                            ui.selectable_value(&mut self.mode, mode, mode.label());
                        }
                    });
                egui::ComboBox::from_id_salt("layout")
                    .selected_text(self.layout.label())
                    .show_ui(ui, |ui| {
                        for layout in Layout::ALL {
                            ui.selectable_value(&mut self.layout, layout, layout.label());
                        }
                    });
                ui.separator();
                ui.label(format!("{:?}", self.state));
                if let Some(mode) = self.active_mode {
                    ui.weak(format!("using {}", mode.label()));
                }
                ui.weak(format!(
                    "decoded {} · dropped {}",
                    self.decoded, self.dropped
                ));
            });
            egui::CollapsingHeader::new("Advanced").show(ui, |ui| {
                ui.add(egui::Slider::new(&mut self.quality, 10..=100).text("JPEG quality"));
                ui.add(egui::Slider::new(&mut self.bandwidth, 4..=20).text("Bandwidth Mbps"));
                ui.add(
                    egui::Slider::new(&mut self.priority_factor, 0..=8).text("Top priority factor"),
                );
            });
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        });

        egui::CentralPanel::default().show(ui, |ui| self.central_screens(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("console_ip", self.console_ip.clone());
        storage.set_string("bind_ip", self.bind_ip.clone());
        storage.set_string("viewer_port", self.viewer_port.to_string());
        storage.set_string(
            "mode",
            StreamMode::ALL
                .iter()
                .position(|mode| *mode == self.mode)
                .unwrap_or(0)
                .to_string(),
        );
        storage.set_string(
            "layout",
            Layout::ALL
                .iter()
                .position(|layout| *layout == self.layout)
                .unwrap_or(0)
                .to_string(),
        );
    }
}
