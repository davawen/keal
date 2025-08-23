use eframe::egui::{self, Layout};
use fork::{fork, Fork};

use std::{os::unix::process::CommandExt, sync::mpsc};
use keal::{arguments::{self, Arguments}, log_time, plugin::{entry::OwnedEntry, Action}, plugin::PluginManager, start_log_time};

use crate::async_manager::{AsyncManager, Event};

mod config;
mod async_manager;

fn main() -> anyhow::Result<()> {
    start_log_time();
    match Arguments::init() {
        Ok(_) => (),
        Err(arguments::Error::Exit) => return Ok(()),
        Err(arguments::Error::UnknownFlag(flag)) => {
            anyhow::bail!("error: unknown flag `{flag}`")
        }
    };

    log_time("reading config");

    let mut theme = config::Theme::default();
    let _config = keal::config::Config::init(&mut theme);

    log_time("starting eframe");

    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native("Keal", native_options, Box::new(|cc| {
        init_eframe(cc);
        Ok(Box::new(Keal::new()))
    }));

    Ok(())
}

fn init_eframe(_cc: &eframe::CreationContext) {
    // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
    // Restore app state using cc.storage (requires the "persistence" feature).
    // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
    // for e.g. egui::PaintCallback.

    // log_time("initializing font");
    // let iosevka = include_bytes!("../../public/iosevka-regular.ttf");
}

struct Keal {
    text: String,
    entries: Vec<OwnedEntry>,
    manager: AsyncManager,
    message_recv: mpsc::Receiver<Message>
}

enum Message {
    Entries(Vec<OwnedEntry>),
    Action(Action)
}

impl Keal {
    fn new() -> Self {
        log_time("initializing keal");

        let (message_send, message_recv) = mpsc::channel();

        let manager = AsyncManager::new(
            nucleo_matcher::Matcher::default(),
            50,
            true,
            message_send
        );

        manager.send(Event::UpdateInput(String::new(), true));

        Keal {
            text: String::new(),
            entries: Vec::new(),
            manager: manager,
            message_recv
        }
    }
}

impl eframe::App for Keal {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let res = ui.text_edit_singleline(&mut self.text);
            if res.changed() {
                self.manager.send(async_manager::Event::UpdateInput(self.text.clone(), true));
            }

            ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for entry in &self.entries {
                    ui.horizontal(|ui| {
                        ui.label(&entry.name);
                        if let Some(comment) = &entry.comment {
                            ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                                ui.label(comment);
                            });
                        }
                    });
                }
            })
        });

        loop {
            let msg = match self.message_recv.try_recv() {
                Ok(msg) => msg,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => panic!("channel disconnected")
            };

            match msg {
                Message::Entries(entries) => self.entries = entries,
                Message::Action(action) => match action {
                    Action::None => (),
                    Action::ChangeInput(new) => {
                        self.manager.with_manager(|m| m.kill());
                        self.text = new.clone();
                        self.manager.send(Event::UpdateInput(new, false));
                    }
                    Action::ChangeQuery(new) => {
                        let new = self.manager.use_manager(|m| m.current().map(
                            |plugin| format!("{} {}", plugin.prefix, new) 
                        )).unwrap_or(new);
                        self.text = new.clone();
                        self.manager.send(Event::UpdateInput(new, false));
                    }
                    Action::WaitAndClose => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    Action::PrintAndClose(s) => {
                        println!("{s}");
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    Action::Exec(mut cmd) => {
                        cmd.0.exec();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    },
                    Action::Fork => match fork().expect("failed to fork") {
                        Fork::Parent(_) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                        Fork::Child => ()
                    }
                }
            }
        }
    }
}
