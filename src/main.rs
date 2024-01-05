use anyhow::Result;
use std::path::PathBuf;

use v4l::control::{Description, Type, Value};

use v4l::Device;

use eframe::egui;

use v4l::context::Node;

fn main() -> Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    )
    .expect("");
    Ok(())
}

struct MyEguiApp {
    selected: Option<PathBuf>,
    selected_name: String,
    cached_device: Option<CachedDevice>,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            cached_device: None,
            selected: None,
            selected_name: String::new(),
        }
    }
}

struct CachedDevice {
    ctrls: Vec<Description>,
    vals: Vec<i64>,
}

impl CachedDevice {
    fn mk(d: &Device) -> Result<Self> {
        let ctrls = d.query_controls()?;
        let mut vals = Vec::with_capacity(ctrls.len());
        for (_i, d) in ctrls.iter().enumerate() {
            vals.push(d.default)
        }
        Ok(Self { ctrls, vals })
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        println!("updating");

        egui::CentralPanel::default().show(ctx, |ui| {
            let oldp = self.selected.clone();

            egui::ComboBox::from_label("Select cam:")
                .selected_text(self.selected_name.to_string())
                .show_ui(ui, |ui| {
                    let vec = v4l::context::enum_devices();
                    for x in vec {
                        let disp = format!(
                            "{} @ {}",
                            x.name().unwrap_or_default(),
                            x.path().to_string_lossy()
                        );
                        ui.selectable_value(&mut self.selected, Some(x.path().to_path_buf()), disp);
                    }
                });

            let Some(path) = self.selected.as_mut() else {
                return;
            };

            if oldp != Some(path.clone()) || ui.button("force refresh cam").clicked() {
                let x = Node::new(path.clone());
                let Ok(d) = Device::with_path(path) else {
                    return;
                };
                self.cached_device = CachedDevice::mk(&d).ok();
                let disp = format!(
                    "{} @ {}",
                    x.name().unwrap_or_default(),
                    x.path().to_string_lossy()
                );

                self.selected_name = disp;
            }

            let apply = Some(Device::new(1).expect("Failed to open device"));
            let force = ui.button("force apply").clicked();
            if let Some(cd) = &mut self.cached_device {
                for (x, val) in cd.ctrls.iter().zip(&mut cd.vals) {
                    let name = &x.name;
                    let ty = x.typ;

                    if ty == Type::CtrlClass {
                        ui.heading(name);
                        ui.separator();
                        continue;
                    };

                    ui.label(name.clone());

                    let old = *val;

                    match ty {
                        Type::Integer => {
                            ui.add(egui::Slider::new(val, x.minimum..=x.maximum));
                        }
                        Type::Boolean => {
                            let mut tmp = *val == 0;
                            ui.checkbox(&mut tmp, "");
                            *val = if tmp { 0 } else { 1 }
                        }
                        Type::Menu => {
                            for (i, item) in x.items.as_ref().unwrap() {
                                ui.radio_value(val, (*i).into(), item.to_string());
                            }
                        }
                        _ => {
                            println!("{name} unhandled: {ty}")
                        }
                    }
                    if let Some(d) = &apply {
                        if *val != old || force {
                            let mut control = d.control(x.id).unwrap();
                            control.value = Value::Integer(*val);
                            match d.set_control(control) {
                                Ok(_) => {
                                    println!("suc set {}", x.name);
                                }
                                Err(_) => {
                                    println!("failed setting {}", x.name);
                                }
                            };
                        }
                    }
                }
            }
        });
    }
}
