use eframe::{egui, epi};

#[derive(Default)]
struct EditorApp {
    editor: vse_rs::Editor,
}

impl epi::App for EditorApp {
    fn name(&self) -> &str {
        "vse-rs standalone"
    }

    fn update(&mut self, ctx: &egui::CtxRef, _: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| self.editor.show(ui));
    }

    fn on_exit(&mut self) {
        println!("{:#?}", self.editor.module())
    }
}

fn main() {
    let mut app = EditorApp::default();
    match std::fs::read_to_string("shader.wgsl") {
        Ok(text) => {
            let module = naga::front::wgsl::parse_str(&text).unwrap();
            app.editor.load_module(module);
        }
        Err(e) => {
            eprintln!("Didn't load shader: {:?}", e)
        }
    }
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
