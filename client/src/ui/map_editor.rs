use std::collections::HashMap;

use common::{
    network::{MapId, MapLayer, MapSettings, TileAnimation, ZoneData},
    TILE_SIZE,
};
use egui::{
    collapsing_header::CollapsingState, menu, Color32, DragValue, Grid, Response, RichText, Ui, WidgetText, Window,
};
use strum::IntoEnumIterator;

use crate::{assets::Assets, game::map::Tile};

use super::tile_selector;

pub fn zone_radio(ui: &mut Ui, selected: bool, title: &str, description: &str) -> Response {
    ui.radio(selected, title).on_hover_ui(|ui| {
        ui.heading(title);
        ui.label(description);
    })
}

fn map_selector(ui: &mut Ui, id: &str, value: &mut Option<MapId>, maps: &HashMap<MapId, String>) {
    let selected_label = if let Some(id) = value {
        if let Some(map) = maps.get(id) {
            format!("{}. {}", id.0, map)
        } else {
            format!("{}.", id.0)
        }
    } else {
        String::from("Disabled")
    };

    egui::ComboBox::from_id_source(id)
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, "Disabled");
            ui.separator();

            let mut items = maps.iter().collect::<Vec<_>>();
            items.sort_unstable_by(|(a, _), (b, _)| a.0.cmp(&b.0));

            for (id, name) in items {
                ui.selectable_value(value, Some(*id), format!("{}. {}", id.0, name));
            }
        });
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Tileset,
    Zones,
    Settings,
    Tools,
}

#[derive(Clone, PartialEq)]
pub enum Wants {
    /// Map editor wishes to exit *while* saving changes
    Save,
    /// Map editor wishes to exit *without* saving changes
    Close,
    /// Map editor wishes to teleport the player to the supplied map
    Warp(MapId),
    /// Map editor wishes to resize the map
    Resize(u32, u32),
}

pub struct MapEditorUpdate {
    pub maps: HashMap<MapId, String>,
    pub width: u32,
    pub height: u32,
    pub id: MapId,
    pub settings: MapSettings,
}

pub struct MapEditor {
    tab: Tab,
    wants: Option<Wants>,

    // map editor
    layer: MapLayer,
    tile_picker: egui::Pos2,
    is_autotile: bool,
    is_tile_animated: bool,
    tile_animation: TileAnimation,

    // zones
    zone_data: ZoneData,

    // settings
    settings: MapSettings,
    id: MapId,
    increment_revision: bool,

    // tools
    maps: HashMap<MapId, String>,
    new_width: u32,
    new_height: u32,
    selected_id: MapId,
}

impl MapEditor {
    pub fn new() -> Self {
        Self {
            tab: Tab::Tileset,
            wants: None,

            // map editor
            layer: MapLayer::Ground,
            tile_picker: egui::pos2(0.0, 0.0),
            is_autotile: false,
            is_tile_animated: false,
            tile_animation: TileAnimation {
                frames: 2,
                duration: 1.0,
                bouncy: false,
            },

            // zones
            zone_data: ZoneData::Blocked,

            // settings
            id: MapId::start(),
            settings: MapSettings::default(),
            increment_revision: true,

            // tools
            maps: HashMap::new(),
            new_width: 0,
            new_height: 0,

            selected_id: MapId::start(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, assets: &Assets, show: &mut bool) {
        if *show {
            Window::new("📝 Map Editor").show(ctx, |ui| self.ui(ui, assets, show));
        }
    }

    pub fn update(&mut self, update: MapEditorUpdate) {
        self.maps = update.maps;
        self.new_width = update.width;
        self.new_height = update.height;
        self.id = update.id;
        self.selected_id = update.id;
        self.settings = update.settings;
    }

    /// The map editor requests a specific thing
    pub fn wants(&mut self) -> Option<Wants> {
        self.wants.take()
    }

    pub fn ui(&mut self, ui: &mut Ui, assets: &Assets, show: &mut bool) {
        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save").clicked() {
                    self.wants = Some(Wants::Save);
                    if self.increment_revision {
                        self.settings.revision += 1;
                    }
                    ui.close_menu();
                    *show = false;
                }
                if ui.button("Exit").clicked() {
                    self.wants = Some(Wants::Close);
                    ui.close_menu();
                    *show = false;
                }
            });

            ui.add_space(6.0);
            ui.separator();

            ui.selectable_value(&mut self.tab, Tab::Tileset, "Tileset");
            ui.selectable_value(&mut self.tab, Tab::Zones, "Zones");
            ui.selectable_value(&mut self.tab, Tab::Settings, "Settings");
            ui.selectable_value(&mut self.tab, Tab::Tools, "Tools");
        });

        ui.separator();

        match self.tab {
            Tab::Tileset => self.show_tileset_tab(ui, assets),
            Tab::Zones => self.show_zone_tab(ui),
            Tab::Settings => self.show_settings_tab(ui, assets),
            Tab::Tools => self.show_tools_tab(ui),
        };
    }

    fn show_tileset_tab(&mut self, ui: &mut Ui, assets: &Assets) {
        let id = ui.make_persistent_id("mapeditor_settings");
        CollapsingState::load_with_default_open(ui.ctx(), id, false)
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
                        });
                    ui.weak("(Press the arrow for more options)");
                });
            })
            .body(|ui| {
                ui.checkbox(&mut self.is_autotile, "Autotile");
                ui.checkbox(&mut self.is_tile_animated, "Animated");
                ui.add_enabled_ui(self.is_tile_animated, |ui| {
                    Grid::new("animation settings").num_columns(2).show(ui, |ui| {
                        ui.label("Duration:");
                        ui.add(
                            DragValue::new(&mut self.tile_animation.duration)
                                .speed(0.01f64)
                                .clamp_range(0f64..=f64::MAX)
                                .suffix("s"),
                        );
                        ui.end_row();

                        ui.label("Frames:");
                        ui.add(
                            DragValue::new(&mut self.tile_animation.frames)
                                .speed(0.1f64)
                                .clamp_range(0f64..=f64::MAX),
                        );
                        ui.end_row();
                    });
                    ui.checkbox(&mut self.tile_animation.bouncy, "Bouncy animation (e.g 1-2-3-2)");
                });
            });

        ui.add_space(3.0);
        tile_selector(
            ui,
            &assets.tileset().egui,
            &mut self.tile_picker,
            egui::Vec2::new(TILE_SIZE as f32, TILE_SIZE as f32),
        );
    }

    fn show_zone_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Zone type");
                    let response = zone_radio(
                        ui,
                        matches!(self.zone_data, ZoneData::Blocked),
                        "Blocked",
                        "Entities are blocked from entering this zone.",
                    );
                    if response.clicked() {
                        self.zone_data = ZoneData::Blocked;
                    }

                    let response = zone_radio(
                        ui,
                        matches!(self.zone_data, ZoneData::Warp(_, _, _)),
                        "Warp",
                        "Teleports a player somewhere else",
                    );
                    if response.clicked() {
                        self.zone_data = ZoneData::Warp(MapId(0), mint::Point2 { x: 0.0, y: 0.0 }, None);
                    }
                });
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Zone data");
                    Grid::new("zone data")
                        .num_columns(2)
                        .show(ui, |ui| match &mut self.zone_data {
                            ZoneData::Blocked => {
                                ui.label("Blocked has no values");
                            }
                            ZoneData::Warp(map_id, position, direction) => {
                                ui.label("Map:");
                                egui::ComboBox::from_id_source("warp map")
                                    .selected_text(format!("{}. {}", map_id.0, self.maps.get(map_id).unwrap()))
                                    .show_ui(ui, |ui| {
                                        let mut items = self.maps.iter().collect::<Vec<_>>();
                                        items.sort_unstable_by(|(a, _), (b, _)| a.0.cmp(&b.0));

                                        for (id, name) in items {
                                            ui.selectable_value(map_id, *id, format!("{}. {}", id.0, name));
                                        }
                                    });
                                ui.end_row();

                                ui.label("Position:");
                                ui.horizontal(|ui| {
                                    ui.label("X: ");
                                    ui.add(DragValue::new(&mut position.x).clamp_range(0.0..=f32::INFINITY));
                                    ui.label("Y: ");
                                    ui.add(DragValue::new(&mut position.y).clamp_range(0.0..=f32::INFINITY));
                                });
                                ui.end_row();

                                ui.label("Direction:");
                                egui::ComboBox::from_id_source("warp direction")
                                    .selected_text(if let Some(direction) = direction {
                                        direction.to_string()
                                    } else {
                                        String::from("Don't change, keep movement")
                                    })
                                    .show_ui(ui, |ui| {
                                        use common::network::Direction;
                                        ui.selectable_value(direction, None, "Don't change, keep movement");
                                        ui.selectable_value(direction, Some(Direction::North), "North");
                                        ui.selectable_value(direction, Some(Direction::East), "East");
                                        ui.selectable_value(direction, Some(Direction::South), "South");
                                        ui.selectable_value(direction, Some(Direction::West), "West");
                                    });
                            }
                        });
                });
            });
        });
    }

    pub fn show_settings_tab(&mut self, ui: &mut Ui, assets: &Assets) {
        let shift = ui.ctx().input().modifiers.shift;

        ui.heading("Map properties");
        Grid::new("properties").num_columns(2).show(ui, |ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.settings.name);
            ui.end_row();

            ui.label("Internal id:");
            ui.label(self.id.0.to_string());
            ui.end_row();

            ui.label("Tileset:");
            egui::ComboBox::from_id_source("tileset")
                .selected_text(&self.settings.tileset)
                .show_ui(ui, |ui| {
                    for tileset in assets.tilesets() {
                        if ui.selectable_label(self.settings.tileset == tileset, tileset).clicked() {
                            self.settings.tileset = tileset.to_owned();
                            assets.set_tileset(tileset).unwrap();
                            self.tile_picker = egui::Pos2::ZERO;
                            ui.close_menu();
                        }
                    }
                });
            ui.end_row();

            ui.label("Music:");
            egui::ComboBox::from_id_source("music")
                .selected_text(if let Some(music) = &self.settings.music {
                    music
                } else {
                    "None"
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.settings.music.is_none(), "None").clicked() {
                        self.settings.music = None;
                        assets.toggle_music(self.settings.music.as_deref());
                    }
                    ui.separator();

                    for item in assets.get_music() {
                        if ui
                            .selectable_label(self.settings.music.as_ref() == Some(&item), &item)
                            .clicked()
                        {
                            self.settings.music = Some(item.clone());
                            assets.toggle_music(self.settings.music.as_deref());
                        }
                    }
                });
            ui.end_row();

            ui.label("Revision:");
            ui.add_enabled(
                shift,
                DragValue::new(&mut self.settings.revision)
                    .clamp_range(0..=u32::MAX)
                    .speed(0.1),
            )
            .on_disabled_hover_ui(|ui| {
                ui.colored_label(Color32::RED, "Manually changing this value may break a lot of things");
                ui.label("Hold shift to enable editing");
            });
            ui.end_row();
        });
        ui.checkbox(&mut self.increment_revision, "Increment revision on save");

        ui.add_space(6.0);

        ui.heading("Edge warps");
        Grid::new("warps").num_columns(3).show(ui, |ui| {
            ui.label("North:");
            map_selector(ui, "north", &mut self.settings.warps.north, &self.maps);
            ui.end_row();

            ui.label("East:");
            map_selector(ui, "east", &mut self.settings.warps.east, &self.maps);
            ui.end_row();

            ui.label("South:");
            map_selector(ui, "south", &mut self.settings.warps.south, &self.maps);
            ui.end_row();

            ui.label("West:");
            map_selector(ui, "west", &mut self.settings.warps.west, &self.maps);
            ui.end_row();
        });

        ui.add_space(3.0);
    }

    pub fn show_tools_tab(&mut self, ui: &mut Ui) {
        let shift = ui.ctx().input().modifiers.shift;

        ui.heading("Teleport");
        ui.label("Select a map and hit ▶, the map editor will close and you will be teleported to it.");
        ui.label("The list contains maps that haven't been created yet, as well as the option to create a new map at the bottom.");
        ui.horizontal(|ui| {
            fn label_text(id: MapId, name: Option<impl AsRef<str>>) -> impl Into<WidgetText> {
                if let Some(name) = name {
                    RichText::new(format!("{}. {}", id.0, name.as_ref()))
                } else {
                    RichText::new(format!("{}. new map", id.0)).italics()
                }
            }
            let selected_text = label_text(self.selected_id, self.maps.get(&self.selected_id));
            egui::ComboBox::from_id_source("map selecter")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    ui.set_min_width(200.0);
                    let max_id = self.maps.keys().fold(0, |acc, k| k.0.max(acc)) + 1;
                    for id in 0..=max_id {
                        let key = MapId(id);
                        let label = label_text(key, self.maps.get(&key));
                        if ui.selectable_label(self.selected_id == key, label).clicked() {
                            self.selected_id = key;
                        }
                    }
                });

            if ui.button("▶").clicked() {
                self.wants = Some(Wants::Warp(self.selected_id));
            }
        });

        ui.add_space(6.0);

        ui.heading("Map size");
        Grid::new("resize").num_columns(2).show(ui, |ui| {
            ui.label("Width:");
            ui.add(
                DragValue::new(&mut self.new_width)
                    .clamp_range(0..=u32::MAX)
                    .speed(0.05)
                    .suffix(" tiles"),
            );
            ui.end_row();

            ui.label("Height:");
            ui.add(
                DragValue::new(&mut self.new_height)
                    .clamp_range(0..=u32::MAX)
                    .speed(0.05)
                    .suffix(" tiles"),
            );
            ui.end_row();

            ui.add_enabled_ui(shift, |ui| {
                let button = ui.button("Save").on_disabled_hover_ui(|ui| {
                    ui.colored_label(
                        Color32::RED,
                        "This will destroy tiles outside of the map and isn't reversable.",
                    );
                    ui.label("Hold shift to enable the save button.");
                });
                if button.clicked() {
                    self.wants = Some(Wants::Resize(self.new_width, self.new_height));
                }
            });
        });

        ui.add_space(6.0);
    }

    pub fn tab(&self) -> Tab {
        self.tab
    }

    pub fn layer(&self) -> MapLayer {
        self.layer
    }

    pub fn tile(&self) -> Tile {
        Tile {
            texture: glam::ivec2(
                self.tile_picker.x as i32 / TILE_SIZE,
                self.tile_picker.y as i32 / TILE_SIZE,
            ),
            autotile: self.is_autotile,
            animation: if self.is_tile_animated {
                Some(self.tile_animation)
            } else {
                None
            },
        }
    }

    pub fn zone_data(&self) -> &ZoneData {
        &self.zone_data
    }

    pub fn map_settings(&self) -> (MapId, &MapSettings) {
        (self.id, &self.settings)
    }
}
