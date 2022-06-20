use egui::*;
use glam::{ivec2};
use onyx_common::{SPRITE_SIZE, network::{TileAnimation, MapLayer, AreaData}, TILE_SIZE};
use strum::IntoEnumIterator;

use crate::{utils::ping_pong, map::Tile, assets::Assets};

pub fn area_radio(ui: &mut Ui, selected: bool, title: &str, description: &str) -> Response {
    ui.radio(selected, title).on_hover_ui(|ui| {
        ui.heading(title);
        ui.label(description);
    })
}

// TODO multiple tile selections
pub fn tile_selector(ui: &mut Ui, texture: &TextureHandle, selected: &mut Pos2, snap: Vec2) {
    ScrollArea::both().show_viewport(ui, |ui, viewport| {
        let clip_rect = ui.clip_rect();

        let margin = ui.visuals().clip_rect_margin;
        let offset = (clip_rect.left_top() - viewport.left_top()) + vec2(margin, margin);
        let texture_size = texture.size_vec2();

        let response = ui.add(Image::new(texture, texture_size).sense(Sense::click()));
        if response.clicked() {
            let pointer = response.interact_pointer_pos().unwrap();
            let position = pointer - offset;
            if position.x >= 0.0 && position.y >= 0.0 && position.x < texture_size.x && position.y < texture_size.y {
                *selected = (snap * (position.to_vec2() / snap).floor()).to_pos2();
            }
        }

        let painter = ui.painter();
        let rect = Rect::from_min_size(*selected + offset, snap);
        painter.rect_stroke(rect, 0., ui.visuals().window_stroke());

        response
    });
}

pub fn sprite_preview(ui: &mut Ui, texture: &TextureHandle, time: f64, sprite: u32) -> Response {
    let sprite_x = (sprite as f64 % 4.0) * 3.0;
    let sprite_y = (sprite as f64 / 4.0).floor() * 4.0;

    // walk left and right 
    let speed = 2.5; // tiles per second
    let loops = 8.0; // how many tiles to walk before rotating

    let animation_speed = 2.0 / speed; // time to complete 1 walk cycle

    let offset_x = ping_pong(time / animation_speed % 1.0, 3) as f64;
    let offset_y = ((time / (animation_speed * loops)) % 4.0).floor();
    //let offset_x = (((time / 0.25).floor() % 4.0).floor() - 1.0).abs();
    // let offset_y = ((time / 4.0).floor() % 4.0).floor();

    let p = vec2((sprite_x + offset_x) as f32 * SPRITE_SIZE as f32, (sprite_y + offset_y) as f32 * SPRITE_SIZE as f32) / texture.size_vec2();
    let size = vec2(SPRITE_SIZE as f32, SPRITE_SIZE as f32) / texture.size_vec2();
    let sprite = Image::new(texture, (SPRITE_SIZE as f32, SPRITE_SIZE as f32))
        .uv(Rect::from_min_size(p.to_pos2(), size));

    ui.add(sprite)
}

#[derive(Clone, Copy, PartialEq)]
pub enum MapEditorTab {
    Tileset,
    Areas,
    Settings
}

#[derive(Clone, Copy, PartialEq)]
pub enum MapEditorWants {
    Nothing,
    SaveMap,
    ReloadMap,
    GetMapSize,
    ResizeMap
}


#[derive(Clone, Copy, PartialEq)]
pub struct MapEditorResponse {
    tab: MapEditorTab,
    wants: MapEditorWants
}

impl MapEditorResponse {
    pub fn wants(&self) -> MapEditorWants {
        self.wants
    }
}

pub struct MapEditor {
    tab: MapEditorTab,

    // map editor
    layer: MapLayer,
    tile_picker: Pos2,
    is_autotile: bool,
    is_tile_animated: bool,
    tile_animation: TileAnimation,

    // areas
    area_data: AreaData,

    // settings
    map_width: u32,
    map_height: u32,
}

impl MapEditor {
    pub fn new() -> Self {
        Self {
            tab: MapEditorTab::Tileset,

            // map editor
            layer: MapLayer::Ground,
            tile_picker: pos2(0.0, 0.0),
            is_autotile: false,
            is_tile_animated: false,
            tile_animation: TileAnimation {
                frames: 2,
                duration: 1.0,
                bouncy: false,
            },

            // area
            area_data: AreaData::Blocked,

            // settings
            map_width: 0,
            map_height: 0,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, assets: &Assets) -> MapEditorResponse {
        let mut wants = MapEditorWants::Nothing;

        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save").clicked() {
                    wants = MapEditorWants::SaveMap;
                    ui.close_menu();
                }
                if ui.button("Exit").clicked() {
                    wants = MapEditorWants::ReloadMap;
                    ui.close_menu();
                }
            });
            ui.add_space(6.0);
            ui.separator();

            ui.selectable_value(&mut self.tab, MapEditorTab::Tileset, "Tileset");
            ui.selectable_value(&mut self.tab, MapEditorTab::Areas, "Areas");
            if ui.selectable_value(&mut self.tab, MapEditorTab::Settings, "Settings").clicked() {
                wants = MapEditorWants::GetMapSize;
            }
        });

        ui.separator();

        let tab_wants = match self.tab {
            MapEditorTab::Tileset => self.show_tileset_tab(ui, assets),
            MapEditorTab::Areas => self.show_area_tab(ui),
            MapEditorTab::Settings => self.show_settings_tab(ui),
        };

        if tab_wants != MapEditorWants::Nothing {
            wants = tab_wants;
        }
        

        MapEditorResponse { tab: self.tab, wants }
    }

    fn show_tileset_tab(&mut self, ui: &mut Ui, assets: &Assets) -> MapEditorWants {
        let id = ui.make_persistent_id("mapeditor_settings");
        collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Layer: ");
                    egui::ComboBox::from_id_source("layer")
                        .selected_text(self.layer.to_string())
                        .show_ui(ui, |ui| {
                            for layer in MapLayer::iter() {
                                if layer == MapLayer::Fringe {
                                    ui.separator();
                                }
                                ui.selectable_value(&mut self.layer, layer, layer.to_string());
                            }
                        }
                    );
                    ui.weak("(Press the arrow for more options)");
                });
            }).body(|ui| {
                ui.checkbox(&mut self.is_autotile, "Autotile");
                ui.checkbox(&mut self.is_tile_animated, "Animated");
                ui.add_enabled_ui(self.is_tile_animated, |ui| {
                    Grid::new("animation settings").num_columns(2).show(ui, |ui| {
                        ui.label("Duration:");
                        ui.add(DragValue::new(&mut self.tile_animation.duration).speed(0.01f64).clamp_range(0f64..=f64::MAX).suffix("s"));
                        ui.end_row();

                        ui.label("Frames:");
                        ui.add(DragValue::new(&mut self.tile_animation.frames).speed(0.1f64).clamp_range(0f64..=f64::MAX));
                        ui.end_row();
                    });
                    ui.checkbox(&mut self.tile_animation.bouncy, "Bouncy animation (e.g 1-2-3-2)");
                });
            });

        ui.add_space(6.0);
        if let Some(texture) = assets.egui.tileset.as_ref() {
            tile_selector(ui, texture, &mut self.tile_picker, Vec2::new(TILE_SIZE as f32, TILE_SIZE as f32));
        };

        MapEditorWants::Nothing
    }

    fn show_area_tab(&mut self, ui: &mut Ui) -> MapEditorWants {
        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Area type");
                    let response = area_radio(ui, matches!(self.area_data, AreaData::Blocked),
                        "Blocked", "Entities are blocked from entering this area.");
                    if response.clicked() {
                        self.area_data = AreaData::Blocked;
                    }

                    let response = area_radio(ui, matches!(self.area_data, AreaData::Log(_)),
                        "Log", "Debug area, sends a message when inside.");
                    if response.clicked() {
                        self.area_data = AreaData::Log(Default::default());
                    }

                });
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Area data");
                    Grid::new("area data").num_columns(2).show(ui, |ui| {
                        match &mut self.area_data {
                            AreaData::Blocked => { ui.label("Blocked has no values"); },
                            AreaData::Log(message) => {
                                ui.label("Greeting:");
                                ui.text_edit_singleline(message);
                                ui.end_row();
                            },
                        }
                    });
                });
            });
        });

        MapEditorWants::Nothing
    }

    pub fn show_settings_tab(&mut self, ui: &mut Ui) -> MapEditorWants {
        let mut wants = MapEditorWants::Nothing;

        ui.group(|ui| {
            ui.heading("Map size");
            Grid::new("resize").num_columns(2).show(ui, |ui| {
                ui.label("Width:");
                ui.add(DragValue::new(&mut self.map_width).clamp_range(0..=u32::MAX).speed(0.05).suffix(" tiles"));
                ui.end_row();

                ui.label("Height:");
                ui.add(DragValue::new(&mut self.map_height).clamp_range(0..=u32::MAX).speed(0.05).suffix(" tiles"));
                ui.end_row();

                let shift = ui.ctx().input().modifiers.shift;
                ui.add_enabled_ui(shift, |ui| {
                    let button = ui.button("Save").on_disabled_hover_ui(|ui| {
                        ui.colored_label(Color32::RED, "This will destroy tiles outside of the map and isn't reversable.");
                        ui.label("Hold shift to enable the save button.");
                    });
                    if button.clicked() {
                        wants = MapEditorWants::ResizeMap;
                    }
                });
            });
        });

        wants
    }

    pub fn tab(&self) -> MapEditorTab {
        self.tab
    }

    pub fn layer(&self) -> MapLayer {
        self.layer
    }

    pub fn map_size(&self) -> (u32, u32) {
        (self.map_width, self.map_height)
    }

    pub fn set_map_size(&mut self, width: u32, height: u32) {
        self.map_width = width;
        self.map_height = height;
    }

    pub fn tile(&self) -> Tile {
        Tile {
            texture: ivec2(self.tile_picker.x as i32 / TILE_SIZE, self.tile_picker.y as i32 / TILE_SIZE),
            autotile: self.is_autotile,
            animation: if self.is_tile_animated {
                Some(self.tile_animation)
            } else {
                None
            },
        }
    }

    pub fn area_data(&self) -> &AreaData {
        &self.area_data
    }
}