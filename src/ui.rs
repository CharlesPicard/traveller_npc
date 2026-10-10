//! eframe/egui front-end: options side panel + character card.

use std::collections::HashMap;

use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Layout, Margin, Pos2, Rect,
    RichText, Sense, Shape, Stroke, TextureHandle, TextureOptions, Ui,
};

use crate::model::{
    format_credits, format_dm, generate, stat_dm, time_seed, GenOptions, Npc, StatMethod, CAREERS,
    SPECIES, STAT_NAMES,
};

// ------------------------------------------------------------ Palette ----
const PAPER: Color32 = Color32::from_rgb(0xEA, 0xEC, 0xE4);
const DARK: Color32 = Color32::from_rgb(0x12, 0x2A, 0x2F);
const DARK2: Color32 = Color32::from_rgb(0x1C, 0x3A, 0x40);
const ACCENT: Color32 = Color32::from_rgb(0x5F, 0xD0, 0xD4);
const LIGHT: Color32 = Color32::from_rgb(0xDC, 0xE8, 0xDF);
const INK: Color32 = Color32::from_rgb(0x14, 0x24, 0x28);
const TEAL_INK: Color32 = Color32::from_rgb(0x1D, 0x6A, 0x70);
const MUTED: Color32 = Color32::from_rgb(0x9F, 0xB4, 0xB6);
const RULE: Color32 = Color32::from_rgb(0xBF, 0xCB, 0xC6);
const WHITE: Color32 = Color32::from_rgb(0xF4, 0xF7, 0xF5);

const LEFT_W: f32 = 520.0;
const RIGHT_W: f32 = 440.0;
const GAP: f32 = 18.0;

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Roster {
    options: GenOptions,
    npcs: Vec<Npc>,
}

/// Progress of a "save card as PNG" request. The window is resized so the whole
/// card is on screen, then an egui screenshot is cropped to the card.
struct PngExport {
    /// Window size in physical pixels before the export.
    prev_size_px: egui::Vec2,
    prev_zoom: f32,
    frames: u32,
    requested: bool,
}

pub struct App {
    opts: GenOptions,
    roster: Vec<Npc>,
    current: usize,
    textures: HashMap<String, Result<TextureHandle, String>>,
    roster_path: String,
    status: String,
    card_size: egui::Vec2,
    export: Option<PngExport>,
    png_requested: bool,
    art_style: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut app = App {
            opts: GenOptions::default(),
            roster: Vec::new(),
            current: 0,
            textures: HashMap::new(),
            roster_path: "npc_roster.json".into(),
            status: String::new(),
            card_size: vec2(LEFT_W + GAP + RIGHT_W + 36.0, 1200.0),
            export: None,
            png_requested: false,
            art_style: "Cinematic science-fiction character portrait, painterly digital art".into(),
        };
        // Start with something on screen.
        app.opts.seed = time_seed();
        app.generate_new();
        app
    }

    fn generate_new(&mut self) {
        if !self.opts.lock_seed {
            self.opts.seed = time_seed();
        }
        let npc = generate(&self.opts);
        self.status = format!("Generated {} (seed {})", npc.name, npc.seed);
        self.roster.push(npc);
        self.current = self.roster.len() - 1;
    }

    fn texture(&mut self, ctx: &egui::Context, path: &str) -> Option<Result<TextureHandle, String>> {
        if path.trim().is_empty() {
            return None;
        }
        let entry = self.textures.entry(path.to_string()).or_insert_with(|| {
            let img = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
            Ok(ctx.load_texture(path, color, TextureOptions::LINEAR))
        });
        Some(entry.clone())
    }

    fn save_roster(&mut self) {
        let data = Roster { options: self.opts.clone(), npcs: self.roster.clone() };
        self.status = match serde_json::to_string_pretty(&data)
            .map_err(|e| e.to_string())
            .and_then(|s| std::fs::write(&self.roster_path, s).map_err(|e| e.to_string()))
        {
            Ok(()) => format!("Saved {} NPCs to {}", self.roster.len(), self.roster_path),
            Err(e) => format!("Save failed: {e}"),
        };
    }

    fn load_roster(&mut self) {
        let res = std::fs::read_to_string(&self.roster_path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<Roster>(&s).map_err(|e| e.to_string()));
        self.status = match res {
            Ok(r) if !r.npcs.is_empty() => {
                self.opts = r.options;
                self.roster = r.npcs;
                self.current = self.roster.len() - 1;
                format!("Loaded {} NPCs from {}", self.roster.len(), self.roster_path)
            }
            Ok(_) => "Roster file is empty".into(),
            Err(e) => format!("Load failed: {e}"),
        };
    }

    fn file_stem(&self) -> Option<String> {
        let npc = self.roster.get(self.current)?;
        Some(
            npc.name
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
                .collect(),
        )
    }

    fn start_png_export(&mut self, ctx: &egui::Context) {
        if self.roster.is_empty() || self.export.is_some() {
            return;
        }
        let ppp = ctx.pixels_per_point();
        let prev_size_px = ctx.input(|i| i.viewport().inner_rect.map(|r| r.size())).unwrap_or(ctx.screen_rect().size()) * ppp;
        let prev_zoom = ctx.zoom_factor();
        // If the card is taller/wider than the monitor, shrink the UI so it still fits.
        if let Some(mon) = ctx.input(|i| i.viewport().monitor_size) {
            let avail = mon - vec2(40.0, 120.0);
            let z = (avail.x / self.card_size.x).min(avail.y / self.card_size.y).min(1.0);
            if z < 1.0 {
                ctx.set_zoom_factor(prev_zoom * z);
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.card_size));
        self.export = Some(PngExport { prev_size_px, prev_zoom, frames: 0, requested: false });
        ctx.request_repaint();
    }

    /// Drives the export state machine; call once per frame, after the card has been laid out.
    fn step_png_export(&mut self, ctx: &egui::Context) {
        let Some(ex) = &mut self.export else { return };
        ex.frames += 1;
        let screenshot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let (prev_px, prev_zoom) = (ex.prev_size_px, ex.prev_zoom);
        let done = |this: &mut Self, msg: String| {
            this.status = msg;
            this.export = None;
            // The zoom change only applies next frame, so convert with the current scale.
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(prev_px / ctx.pixels_per_point()));
            ctx.set_zoom_factor(prev_zoom);
        };
        if let Some(img) = screenshot {
            let ppp = ctx.pixels_per_point();
            let rect = Rect::from_min_size(Pos2::ZERO, self.card_size);
            let crop = img.region(&rect, Some(ppp));
            let msg = match (self.file_stem(), image::RgbaImage::from_raw(
                crop.width() as u32, crop.height() as u32, crop.as_raw().to_vec()))
            {
                (Some(stem), Some(buf)) => {
                    let path = format!("{stem}.png");
                    match buf.save(&path) {
                        Ok(()) => format!("Exported {path} ({}×{})", buf.width(), buf.height()),
                        Err(e) => format!("PNG export failed: {e}"),
                    }
                }
                _ => "PNG export failed".into(),
            };
            done(self, msg);
            return;
        }
        let fits = ctx.screen_rect().size().x + 1.0 >= self.card_size.x && ctx.screen_rect().size().y + 1.0 >= self.card_size.y;
        if !ex.requested && fits && ex.frames >= 4 {
            ex.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        } else if ex.frames > 90 {
            let msg = format!(
                "PNG export failed: window is {:.0}×{:.0}, card needs {:.0}×{:.0}",
                ctx.screen_rect().width(), ctx.screen_rect().height(), self.card_size.x, self.card_size.y);
            done(self, msg);
        }
        ctx.request_repaint();
    }

    fn export_markdown(&mut self) {
        let Some(file) = self.file_stem() else { return };
        let Some(npc) = self.roster.get(self.current) else { return };
        let path = format!("{file}.md");
        self.status = match std::fs::write(&path, npc.to_markdown()) {
            Ok(()) => format!("Exported {path}"),
            Err(e) => format!("Export failed: {e}"),
        };
    }

    // ------------------------------------------------------ Side panel ----
    fn side_panel(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(6.0);
            ui.heading("Traveller NPC");
            ui.separator();

            ui.label(RichText::new("GENERATION").small().color(ACCENT));
            egui::Grid::new("opts").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label("Name");
                ui.add(egui::TextEdit::singleline(&mut self.opts.name).hint_text("random"));
                ui.end_row();

                ui.label("Species");
                let sel = self.opts.species.map(|i| SPECIES[i]).unwrap_or("Random");
                egui::ComboBox::from_id_salt("species").selected_text(sel).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.opts.species, None, "Random");
                    for (i, s) in SPECIES.iter().enumerate() {
                        ui.selectable_value(&mut self.opts.species, Some(i), *s);
                    }
                });
                ui.end_row();

                ui.label("Career");
                let sel = self.opts.career.map(|i| CAREERS[i].name).unwrap_or("Random");
                egui::ComboBox::from_id_salt("career").selected_text(sel).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.opts.career, None, "Random");
                    for (i, c) in CAREERS.iter().enumerate() {
                        ui.selectable_value(&mut self.opts.career, Some(i), c.name);
                    }
                });
                ui.end_row();

                ui.label("Terms");
                ui.horizontal(|ui| {
                    let mut random = self.opts.terms.is_none();
                    ui.checkbox(&mut random, "Random");
                    if random {
                        self.opts.terms = None;
                    } else {
                        let mut t = self.opts.terms.unwrap_or(5);
                        ui.add(egui::Slider::new(&mut t, 1..=7));
                        self.opts.terms = Some(t);
                    }
                });
                ui.end_row();

                ui.label("Stats");
                egui::ComboBox::from_id_salt("method")
                    .selected_text(self.opts.method.label())
                    .show_ui(ui, |ui| {
                        for m in StatMethod::ALL {
                            ui.selectable_value(&mut self.opts.method, m, m.label());
                        }
                    });
                ui.end_row();

                ui.label("Seed");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.opts.seed).speed(1.0));
                    ui.checkbox(&mut self.opts.lock_seed, "Lock");
                });
                ui.end_row();
            });
            ui.checkbox(&mut self.opts.aging, "Aging checks (term 4+)");
            ui.checkbox(&mut self.opts.end_on_mishap, "Career ends on mishap");

            ui.add_space(4.0);
            ui.label("Portrait image for new NPCs");
            ui.add(
                egui::TextEdit::singleline(&mut self.opts.portrait_path)
                    .hint_text("path to .png / .jpg (or drop a file on the window)"),
            );

            ui.add_space(8.0);
            let gen = ui.add_sized(
                [ui.available_width(), 36.0],
                egui::Button::new(RichText::new("⚄  Generate NPC").size(17.0)).fill(DARK2),
            );
            if gen.clicked() {
                self.generate_new();
            }
            ui.horizontal(|ui| {
                if ui.button("Reroll roleplaying").clicked() {
                    if let Some(n) = self.roster.get_mut(self.current) {
                        n.reroll_roleplay(time_seed());
                    }
                }
                if ui.button("Reroll same options").on_hover_text("New seed, keep everything else").clicked() {
                    let locked = self.opts.lock_seed;
                    self.opts.lock_seed = false;
                    self.generate_new();
                    self.opts.lock_seed = locked;
                }
            });

            // ------------------------------------------ Current NPC edits
            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new("EDIT CURRENT NPC").small().color(ACCENT));
            let portrait_default = self.opts.portrait_path.clone();
            if let Some(n) = self.roster.get_mut(self.current) {
                egui::Grid::new("edit").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut n.name);
                    ui.end_row();
                    ui.label("Trait");
                    ui.text_edit_singleline(&mut n.trait_);
                    ui.end_row();
                    ui.label("Wants");
                    ui.text_edit_singleline(&mut n.wants);
                    ui.end_row();
                    ui.label("Hook");
                    ui.text_edit_singleline(&mut n.hook);
                    ui.end_row();
                    ui.label("Quote");
                    ui.add(egui::TextEdit::multiline(&mut n.quote).desired_rows(2));
                    ui.end_row();
                    ui.label("Portrait");
                    ui.text_edit_singleline(&mut n.portrait_path);
                    ui.end_row();
                });
                if ui.button("Use side-panel portrait on this NPC").clicked() {
                    n.portrait_path = portrait_default;
                }
            }

            // ------------------------------------------------ Roster
            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new(format!("ROSTER ({})", self.roster.len())).small().color(ACCENT));
            let mut remove = None;
            for (i, n) in self.roster.iter().enumerate().rev() {
                ui.horizontal(|ui| {
                    let label = format!("{} — {} {}", n.name, n.species, n.rank_title);
                    if ui.selectable_label(i == self.current, label).clicked() {
                        self.current = i;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("✖").on_hover_text("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                });
            }
            if let Some(i) = remove {
                self.roster.remove(i);
                if self.current >= self.roster.len() {
                    self.current = self.roster.len().saturating_sub(1);
                }
            }

            // ------------------------------------------------ Files
            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new("FILES").small().color(ACCENT));
            ui.horizontal(|ui| {
                ui.label("Roster");
                ui.text_edit_singleline(&mut self.roster_path);
            });
            ui.horizontal(|ui| {
                if ui.button("Save roster").clicked() {
                    self.save_roster();
                }
                if ui.button("Load roster").clicked() {
                    self.load_roster();
                }
                if ui.button("Export .md").clicked() {
                    self.export_markdown();
                }
            });
            if ui
                .add_sized([ui.available_width(), 28.0], egui::Button::new("🖼  Export card to PNG"))
                .on_hover_text("Saves the full character card as <name>.png in the working directory")
                .clicked()
            {
                self.png_requested = true;
            }
            if !self.status.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(&self.status).small().color(MUTED));
            }
        });
    }

    // ------------------------------------------------ Appearance panel ----
    fn appearance_panel(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(6.0);
            ui.heading("Appearance");
            ui.separator();
            let Some(n) = self.roster.get_mut(self.current) else {
                ui.label("No NPC selected.");
                return;
            };
            ui.label(RichText::new("PHYSICAL DESCRIPTION").small().color(ACCENT));
            let w = ui.available_width();
            let fields: [(&str, &mut String); 8] = [
                ("Body", &mut n.look.body),
                ("Skin / fur", &mut n.look.skin),
                ("Hair / mane", &mut n.look.hair),
                ("Eyes", &mut n.look.eyes),
                ("Marks", &mut n.look.mark),
                ("Attire", &mut n.look.attire),
                ("Expression", &mut n.look.expression),
                ("Setting", &mut n.look.setting),
            ];
            for (label, value) in fields {
                ui.label(label);
                ui.add(egui::TextEdit::multiline(value).desired_rows(2).desired_width(w));
            }
            if ui.button("Reroll appearance").clicked() {
                n.reroll_look(time_seed());
            }

            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new("IMAGE PROMPT").small().color(ACCENT));
            ui.label("Art style");
            ui.add(egui::TextEdit::multiline(&mut self.art_style).desired_rows(2).desired_width(w));
            let prompt = n.image_prompt(&self.art_style);
            ui.add_space(4.0);
            ui.add(egui::TextEdit::multiline(&mut prompt.as_str()).desired_rows(12).desired_width(w));
            if ui.add_sized([w, 28.0], egui::Button::new("📋  Copy prompt")).clicked() {
                ui.ctx().copy_text(prompt);
                self.status = "Prompt copied to clipboard".into();
            }
        });
    }

    // -------------------------------------------------------- The card ----
    fn card(&mut self, ui: &mut Ui) {
        let Some(npc) = self.roster.get(self.current).cloned() else {
            ui.centered_and_justified(|ui| ui.label("Generate an NPC from the side panel."));
            return;
        };
        let tex = self.texture(ui.ctx(), &npc.portrait_path);

        let resp = egui::Frame::default()
            .fill(PAPER)
            .inner_margin(Margin::same(18))
            .corner_radius(cr(4))
            .show(ui, |ui| {
                ui.set_width(LEFT_W + GAP + RIGHT_W);
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    ui.vertical(|ui| {
                        ui.set_width(LEFT_W);
                        portrait(ui, &npc, tex);
                        ui.add_space(14.0);
                        career_log(ui, &npc);
                    });
                    ui.vertical(|ui| {
                        ui.set_width(RIGHT_W);
                        name_banner(ui, &npc);
                        ui.add_space(10.0);
                        header(ui, &npc);
                        ui.add_space(14.0);
                        stats(ui, &npc);
                        ui.add_space(10.0);
                        skills(ui, &npc);
                        ui.add_space(14.0);
                        roleplaying(ui, &npc);
                    });
                });
            });
        self.card_size = resp.response.rect.size();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drag-and-drop an image onto the window to set the current portrait.
        let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if let Some(p) = dropped.first() {
            let p = p.to_string_lossy().to_string();
            if let Some(n) = self.roster.get_mut(self.current) {
                n.portrait_path = p.clone();
            }
            self.opts.portrait_path = p;
        }

        let exporting = self.export.is_some();
        if !exporting {
            egui::SidePanel::left("options")
                .resizable(true)
                .default_width(320.0)
                .min_width(260.0)
                .show(ctx, |ui| self.side_panel(ui));
            egui::SidePanel::right("appearance")
                .resizable(true)
                .default_width(340.0)
                .min_width(260.0)
                .show(ctx, |ui| self.appearance_panel(ui));
        }
        if self.png_requested {
            self.png_requested = false;
            self.start_png_export(ctx);
        }

        // While exporting, the card sits flush in the top-left corner with no scrolling.
        let margin = if exporting { Margin::ZERO } else { Margin::same(16) };
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::from_rgb(0x0B, 0x16, 0x19)).inner_margin(margin))
            .show(ctx, |ui| {
                if exporting {
                    self.card(ui);
                } else {
                    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| self.card(ui));
                }
            });
        self.step_png_export(ctx);
    }
}

// ------------------------------------------------------ Card sections ----

fn portrait(ui: &mut Ui, npc: &Npc, tex: Option<Result<TextureHandle, String>>) {
    let (rect, _) = ui.allocate_exact_size(vec2(LEFT_W, 760.0), Sense::hover());
    let p = ui.painter_at(rect);

    match &tex {
        Some(Ok(t)) => {
            // "Cover" fit, biased toward the top so faces stay in frame.
            let [w, h] = t.size();
            let img_aspect = w as f32 / h as f32;
            let box_aspect = rect.width() / rect.height();
            let uv = if img_aspect > box_aspect {
                let u = box_aspect / img_aspect;
                Rect::from_min_max(pos2((1.0 - u) / 2.0, 0.0), pos2((1.0 + u) / 2.0, 1.0))
            } else {
                let v = img_aspect / box_aspect;
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, v))
            };
            p.image(t.id(), rect, uv, Color32::WHITE);
        }
        _ => {
            // Placeholder: layered "skyline" so the card still looks right.
            p.rect_filled(rect, cr(0), Color32::from_rgb(0x9C, 0xB6, 0xC4));
            let base = rect.bottom() - 240.0;
            for (i, (x, w, hgt)) in [(30.0, 40.0, 260.0), (90.0, 26.0, 330.0), (360.0, 50.0, 420.0),
                (420.0, 30.0, 300.0), (470.0, 36.0, 360.0), (200.0, 60.0, 180.0)].iter().enumerate()
            {
                let shade = 0x55 + (i as u8 * 9);
                p.rect_filled(
                    Rect::from_min_size(pos2(rect.left() + x, base - hgt), vec2(*w, *hgt + 240.0)),
                    cr(2),
                    Color32::from_rgb(shade, shade + 0x18, shade + 0x24),
                );
            }
            // Silhouette
            let c = pos2(rect.center().x - 20.0, rect.top() + 250.0);
            p.circle_filled(c, 90.0, DARK2);
            p.add(Shape::convex_polygon(
                vec![pos2(c.x - 210.0, rect.bottom()), pos2(c.x - 120.0, c.y + 130.0),
                     pos2(c.x + 120.0, c.y + 130.0), pos2(c.x + 210.0, rect.bottom())],
                DARK2,
                Stroke::NONE,
            ));
            let msg = match &tex {
                Some(Err(e)) => format!("Couldn't load portrait:\n{e}"),
                _ => "No portrait — drop an image on the window\nor set a path in the side panel".into(),
            };
            p.text(pos2(rect.center().x, rect.top() + 40.0), Align2::CENTER_TOP, msg,
                FontId::proportional(15.0), DARK);
        }
    }

    // Quote overlay, bottom-left.
    let qrect = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 110.0), vec2(400.0, 96.0));
    p.add(Shape::convex_polygon(
        vec![qrect.left_top(), pos2(qrect.right() - 30.0, qrect.top()), qrect.right_bottom(), qrect.left_bottom()],
        Color32::from_rgba_unmultiplied(0x0E, 0x22, 0x27, 225),
        Stroke::NONE,
    ));
    p.line_segment([pos2(qrect.left() + 14.0, qrect.top() + 14.0), pos2(qrect.left() + 14.0, qrect.bottom() - 14.0)],
        Stroke::new(2.0, ACCENT));
    let galley = p.layout(
        format!("\u{201c}{}\u{201d}", npc.quote),
        FontId::proportional(17.0),
        WHITE,
        qrect.width() - 70.0,
    );
    let y = qrect.center().y - galley.size().y / 2.0;
    p.galley(pos2(qrect.left() + 28.0, y), galley, WHITE);
}

fn header(ui: &mut Ui, npc: &Npc) {
    egui::Frame::default()
        .fill(DARK)
        .corner_radius(cr(8))
        .inner_margin(Margin::symmetric(20, 18))
        .show(ui, |ui| {
            ui.set_width(RIGHT_W - 40.0);
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
                emblem(ui, r.center(), 32.0);
                ui.add_space(8.0);
                ui.add(egui::Label::new(RichText::new(npc.title()).size(32.0).strong().color(WHITE)).wrap());
            });
            ui.add_space(6.0);
            let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 3.0), Sense::hover());
            ui.painter().rect_filled(line, cr(1), ACCENT);
            ui.add_space(8.0);
            let big = |s: String| RichText::new(s).size(20.0).color(WHITE);
            ui.horizontal(|ui| {
                ui.label(big(format!("{}   •   Age {}   •   {} terms", npc.career, npc.age, npc.terms)));
            });
            ui.horizontal(|ui| {
                ui.label(big("UPP".into()));
                ui.label(RichText::new(npc.upp()).size(20.0).strong().color(ACCENT));
                ui.label(big(format!("  •   {}", format_credits(npc.credits))));
            });
        });
}

/// Character name set above the header: a small designation tag, the name in
/// large caps beside an accent bar, and a rule ending in a diamond.
fn name_banner(ui: &mut Ui, npc: &Npc) {
    let name = npc.name.to_uppercase();
    let galley = ui.painter().layout(name, FontId::proportional(44.0), DARK, RIGHT_W - 28.0);
    let h = galley.size().y + 50.0;
    let (r, _) = ui.allocate_exact_size(vec2(RIGHT_W, h), Sense::hover());
    let p = ui.painter();
    // Accent bar with a chamfered top.
    p.add(Shape::convex_polygon(
        vec![pos2(r.left(), r.top() + 4.0), pos2(r.left() + 10.0, r.top() + 4.0),
             pos2(r.left() + 10.0, r.bottom() - 12.0), pos2(r.left(), r.bottom() - 12.0)],
        ACCENT,
        Stroke::NONE,
    ));
    p.text(pos2(r.left() + 28.0, r.top() + 4.0), Align2::LEFT_TOP, "D E S I G N A T I O N",
        FontId::proportional(13.0), TEAL_INK);
    p.galley(pos2(r.left() + 26.0, r.top() + 22.0), galley, DARK);
    let y = r.bottom() - 6.0;
    p.line_segment([pos2(r.left() + 28.0, y), pos2(r.right() - 14.0, y)], Stroke::new(2.0, DARK));
    let d = pos2(r.right() - 6.0, y);
    p.add(Shape::convex_polygon(
        vec![pos2(d.x - 8.0, d.y), pos2(d.x, d.y - 6.0), pos2(d.x + 8.0, d.y), pos2(d.x, d.y + 6.0)],
        ACCENT,
        Stroke::NONE,
    ));
}

/// Stylised planet-and-ring emblem.
fn emblem(ui: &Ui, c: Pos2, r: f32) {
    let p = ui.painter();
    p.circle_stroke(c, r, Stroke::new(3.0, ACCENT));
    p.circle_filled(c, r * 0.55, ACCENT);
    p.circle_filled(c + vec2(r * 0.18, -r * 0.12), r * 0.2, DARK);
    let pts: Vec<Pos2> = (0..=40)
        .map(|i| {
            let t = i as f32 / 40.0 * std::f32::consts::TAU;
            let (x, y) = (t.cos() * r * 1.15, t.sin() * r * 0.35);
            let a = -0.45f32;
            c + vec2(x * a.cos() - y * a.sin(), x * a.sin() + y * a.cos())
        })
        .collect();
    p.add(Shape::line(pts, Stroke::new(2.5, ACCENT)));
}

fn stats(ui: &mut Ui, npc: &Npc) {
    for (i, name) in STAT_NAMES.iter().enumerate() {
        let v = npc.stats[i];
        let (r, _) = ui.allocate_exact_size(vec2(RIGHT_W, 50.0), Sense::hover());
        let p = ui.painter();
        // Dark bar with chamfered right edge.
        p.add(Shape::convex_polygon(
            vec![r.left_top(), pos2(r.right() - 18.0, r.top()), pos2(r.right(), r.center().y),
                 pos2(r.right() - 18.0, r.bottom()), r.left_bottom()],
            DARK,
            Stroke::NONE,
        ));
        // Light value tab.
        let x0 = r.left() + 130.0;
        p.add(Shape::convex_polygon(
            vec![pos2(x0 + 20.0, r.top() + 5.0), pos2(x0 + 120.0, r.top() + 5.0),
                 pos2(x0 + 120.0, r.bottom() - 5.0), pos2(x0, r.bottom() - 5.0)],
            LIGHT,
            Stroke::NONE,
        ));
        p.text(pos2(r.left() + 22.0, r.center().y), Align2::LEFT_CENTER, *name,
            FontId::proportional(26.0), WHITE);
        p.text(pos2(x0 + 64.0, r.center().y), Align2::CENTER_CENTER, v.to_string(),
            FontId::proportional(26.0), TEAL_INK);
        p.text(pos2(x0 + 150.0, r.center().y), Align2::LEFT_CENTER, "DM",
            FontId::proportional(18.0), MUTED);
        p.text(pos2(r.right() - 40.0, r.center().y), Align2::RIGHT_CENTER, format_dm(stat_dm(v)),
            FontId::proportional(24.0), WHITE);
        ui.add_space(6.0);
    }
}

fn section_header(ui: &mut Ui, width: f32, icon: &str, title: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(width, 46.0), Sense::hover());
    let p = ui.painter();
    p.add(Shape::convex_polygon(
        vec![r.left_bottom(), pos2(r.left(), r.top() + 8.0), pos2(r.left() + 8.0, r.top()),
             pos2(r.right() - 22.0, r.top()), pos2(r.right(), r.top() + 22.0), r.right_bottom()],
        DARK,
        Stroke::NONE,
    ));
    p.text(pos2(r.left() + 20.0, r.center().y), Align2::LEFT_CENTER, icon, FontId::proportional(24.0), WHITE);
    p.text(pos2(r.left() + 58.0, r.center().y), Align2::LEFT_CENTER, title, FontId::proportional(23.0), WHITE);
}

fn panel_body<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::default()
        .fill(PAPER)
        .stroke(Stroke::new(2.0, DARK))
        .corner_radius(CornerRadius { nw: 0, ne: 0, sw: 8, se: 8 })
        .inner_margin(Margin::symmetric(22, 12))
        .show(ui, |ui| {
            ui.set_width(width - 44.0);
            add(ui)
        })
        .inner
}

fn skills(ui: &mut Ui, npc: &Npc) {
    ui.spacing_mut().item_spacing.y = 0.0;
    section_header(ui, RIGHT_W, "⚙", "SKILLS");
    panel_body(ui, RIGHT_W, |ui| {
        let n = npc.skills.len();
        for (i, (name, lvl)) in npc.skills.iter().enumerate() {
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
            let p = ui.painter();
            p.text(pos2(r.left(), r.center().y), Align2::LEFT_CENTER, name, FontId::proportional(20.0), INK);
            p.text(pos2(r.right(), r.center().y), Align2::RIGHT_CENTER, lvl.to_string(),
                FontId::proportional(22.0), TEAL_INK);
            if i + 1 < n {
                p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0, RULE));
            }
        }
    });
    ui.spacing_mut().item_spacing.y = 4.0;
}

fn roleplaying(ui: &mut Ui, npc: &Npc) {
    ui.spacing_mut().item_spacing.y = 0.0;
    section_header(ui, RIGHT_W, "🎭", "ROLEPLAYING");
    panel_body(ui, RIGHT_W, |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        let rows = [("★", "Trait:", &npc.trait_), ("◎", "Wants:", &npc.wants), ("⚠", "Hook:", &npc.hook)];
        for (i, (icon, label, value)) in rows.iter().enumerate() {
            ui.add_space(6.0);
            ui.horizontal_top(|ui| {
                ui.add_sized([34.0, 28.0], egui::Label::new(RichText::new(*icon).size(24.0).color(DARK)));
                ui.vertical(|ui| {
                    ui.label(RichText::new(*label).size(19.0).strong().color(TEAL_INK));
                    ui.add(egui::Label::new(RichText::new(value.as_str()).size(19.0).color(INK)).wrap());
                });
            });
            ui.add_space(6.0);
            if i < rows.len() - 1 {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                ui.painter().rect_filled(r, cr(0), RULE);
            }
        }
    });
}

fn career_log(ui: &mut Ui, npc: &Npc) {
    ui.spacing_mut().item_spacing.y = 0.0;
    section_header(ui, LEFT_W, "📖", "CAREER LOG");
    panel_body(ui, LEFT_W, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        for (i, t) in npc.log.iter().enumerate() {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                egui::Frame::default()
                    .fill(DARK)
                    .corner_radius(cr(3))
                    .inner_margin(Margin::symmetric(12, 3))
                    .show(ui, |ui| ui.label(RichText::new(format!("Term {}", t.term)).size(17.0).color(WHITE)));
                ui.add_space(10.0);
                ui.label(RichText::new(format!("(age {}–{})", t.age_start, t.age_end)).size(16.0).color(INK));
            });
            for e in &t.events {
                ui.horizontal_top(|ui| {
                    ui.add_space(24.0);
                    ui.label(RichText::new("•").size(15.0).color(INK));
                    ui.add(egui::Label::new(RichText::new(e).size(15.0).color(INK)).wrap());
                });
            }
            ui.add_space(6.0);
            if i + 1 < npc.log.len() {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                ui.painter().rect_filled(r, cr(0), RULE);
            }
        }
        ui.add_space(8.0);
        egui::Frame::default()
            .fill(DARK)
            .corner_radius(cr(4))
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(">_").monospace().size(17.0).color(ACCENT));
                    ui.add_space(8.0);
                    ui.label(RichText::new(npc.muster_line()).size(16.0).color(WHITE));
                });
            });
    });
}
