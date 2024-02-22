use std::collections::{BTreeMap, HashMap};

use common::{
    network::{MapLayer, MapSettings, TileAnimation, ZoneData, MapId},
    TILE_SIZE,
};
use ggegui::egui::{*, collapsing_header::CollapsingState};
use ggez::glam::IVec2;
use strum::IntoEnumIterator;

use crate::{AssetCache, data::Tile};

use super::tile_selector;

pub fn zone_radio(ui: &mut Ui, selected: bool, title: &str, description: &str) -> Response {
    ui.radio(selected, title).on_hover_ui(|ui| {
        ui.heading(title);
        ui.label(description);
    })
}

fn map_option_selector(ui: &mut Ui, id_source: &str, value: &mut Option<MapId>, maps: &BTreeMap<MapId, String>) {
    let selected_label = match value {
        Some(id) => format!("{}. {}", id.0, maps[id]),
        None => String::from("Disabled"),
    };

    ComboBox::from_id_source(id_source)
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, "Disabled");
            ui.separator();

            for (id, name) in maps {
                // literally just Option::contains
                let selected = value.as_ref() == Some(id);

                if ui.selectable_label(selected, format!("{}. {}", id.0, name)).clicked() {
                    *value = Some(*id);
                }
            }
        });
}

fn map_selector(ui: &mut Ui, id_source: &str, value: &mut MapId, maps: &BTreeMap<MapId, String>) {
    ComboBox::from_id_source(id_source)
        .selected_text(format!("{}. {}", value.0, maps.get(value).map(AsRef::as_ref).unwrap_or("")))
        .show_ui(ui, |ui| {
            for (id, name) in maps {
                if ui.selectable_label(value == id, format!("{}. {}", id.0, name)).clicked() {
                    *value = *id;
                }
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

#[derive(Clone)]
pub enum Wants {
    /// Map editor wishes to exit *while* saving changes
    Save,
    /// Map editor wishes to exit *without* saving changes
    Close,
    /// Map editor wishes to teleport the player to the supplied map
    Warp(MapId),
    /// Map editor wishes to resize the map
    Resize(u32, u32),
    /// Map editor wishes to fill the layer with a tile
    Fill(MapLayer, Option<Tile>),
}

pub struct MapEditor {
    tab: Tab,
    wants: Option<Wants>,

    // map editor
    layer: MapLayer,
    tile_picker: Pos2,
    is_autotile: bool,
    is_tile_animated: bool,
    tile_animation: TileAnimation,

    // zones
    zone_data: ZoneData,

    // settings
    settings: MapSettings,
    id: MapId,

    // tools
    maps: BTreeMap<MapId, String>,
    new_width: u32,
    new_height: u32,
    selected_map: MapId,
}

impl MapEditor {
    pub fn new() -> Self {
        Self {
            tab: Tab::Tileset,
            wants: None,

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

            // zones
            zone_data: ZoneData::Blocked,

            // settings
            id: MapId::default(),
            settings: MapSettings::default(),

            // tools
            maps: BTreeMap::new(),
            new_width: 0,
            new_height: 0,

            selected_map: MapId::default(),
        }
    }

    pub fn show(&mut self, ctx: &Context, assets: &mut AssetCache, show: &mut bool) {
        if *show {
            Window::new("📝 Map Editor").show(ctx, |ui| self.ui(ui, assets, show));
        }
    }

    pub fn update(&mut self, maps: HashMap<MapId, String>, width: u32, height: u32, id: MapId, settings: MapSettings) {
        self.new_width = width;
        self.new_height = height;
        self.id = id;
        self.selected_map = id;
        self.settings = settings;

        self.maps = maps.into_iter().collect::<BTreeMap<_, _>>();
    }

    /// The map editor requests a specific thing
    pub fn wants(&mut self) -> Option<Wants> {
        self.wants.take()
    }

    pub fn ui(&mut self, ui: &mut Ui, assets: &mut AssetCache, show: &mut bool) {
        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save").clicked() {
                    self.wants = Some(Wants::Save);
                    ui.close_menu();
                    *show = false;
                }
                if ui.button("Exit").clicked() {
                    self.wants = Some(Wants::Close);
                    ui.close_menu();
                    *show = false;
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Fill layer").clicked() {
                    self.wants = Some(Wants::Fill(self.layer, Some(self.tile())));
                    ui.close_menu();
                }
                if ui.button("Clear layer").clicked() {
                    self.wants = Some(Wants::Fill(self.layer, None));
                    ui.close_menu();
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

    fn show_tileset_tab(&mut self, ui: &mut Ui, assets: &mut AssetCache) {
        let id = ui.make_persistent_id("mapeditor_settings");
        CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Layer: ");
                    ComboBox::from_id_source("layer")
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
                                .clamp_range(0.0..=f64::MAX)
                                .suffix("s"),
                        );
                        ui.end_row();

                        ui.label("Frames:");
                        ui.add(
                            DragValue::new(&mut self.tile_animation.frames)
                                .speed(0.1f64)
                                .clamp_range(0.0..=f64::MAX),
                        );
                        ui.end_row();
                    });
                    ui.checkbox(&mut self.tile_animation.bouncy, "Bouncy animation (e.g 1-2-3-2)");
                });
            });

        ui.add_space(3.0);
        let tileset = &assets.tileset_egui;
        let size = vec2(assets.tileset.width() as f32, assets.tileset.height() as f32);
        tile_selector(
            ui,
            tileset.id(),
            size.into(),
            &mut self.tile_picker,
            Vec2::new(TILE_SIZE as f32, TILE_SIZE as f32),
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
                        self.zone_data = ZoneData::Warp(MapId::default(), glam::Vec2::ZERO.into(), None);
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
                            ZoneData::Warp(mut map_id, position, direction) => {
                                ui.label("Map:");
                                map_selector(ui, "zone_map", &mut map_id, &self.maps);
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
                                ComboBox::from_id_source("warp direction")
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

    pub fn show_settings_tab(&mut self, ui: &mut Ui, _assets: &mut AssetCache) {
        ui.heading("Map properties");
        Grid::new("properties").num_columns(2).show(ui, |ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.settings.name);
            ui.end_row();

            ui.label("Internal id:");
            ui.add_enabled(false, DragValue::new(&mut self.id.0));
            ui.end_row();

            ui.label("Tileset:");
            // ComboBox::from_id_source("tileset")
            //     .selected_text(&self.settings.tileset)
            //     .show_ui(ui, |ui| {
            //         for tileset in assets.tilesets() {
            //             if ui.selectable_label(self.settings.tileset == tileset, tileset).clicked() {
            //                 self.settings.tileset = tileset.to_owned();
            //                 assets.set_tileset(tileset).unwrap();
            //                 self.tile_picker = egui::Pos2::ZERO;
            //                 ui.close_menu();
            //             }
            //         }
            //     });
            ui.end_row();

            ui.label("Music:");
            ComboBox::from_id_source("music")
                .selected_text(if let Some(music) = &self.settings.music {
                    music
                } else {
                    "None"
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.settings.music.is_none(), "None").clicked() {
                        self.settings.music = None;
                        // assets.toggle_music(self.settings.music.as_deref());
                    }
                    ui.separator();

                    // for item in assets.get_music() {
                    //     if ui
                    //         .selectable_label(self.settings.music.as_ref() == Some(&item), &item)
                    //         .clicked()
                    //     {
                    //         self.settings.music = Some(item.clone());
                    //         assets.toggle_music(self.settings.music.as_deref());
                    //     }
                    // }
                });
            ui.end_row();
        });

        ui.add_space(6.0);

        ui.heading("Edge warps");
        Grid::new("warps").num_columns(3).show(ui, |ui| {
            ui.label("North:");
            map_option_selector(ui, "north", &mut self.settings.warps.north, &self.maps);
            ui.end_row();

            ui.label("East:");
            map_option_selector(ui, "east", &mut self.settings.warps.east, &self.maps);
            ui.end_row();

            ui.label("South:");
            map_option_selector(ui, "south", &mut self.settings.warps.south, &self.maps);
            ui.end_row();

            ui.label("West:");
            map_option_selector(ui, "west", &mut self.settings.warps.west, &self.maps);
            ui.end_row();
        });

        ui.add_space(3.0);
    }

    pub fn show_tools_tab(&mut self, ui: &mut Ui) {
        let shift = ui.ctx().input(|i| i.modifiers.shift);

        ui.heading("Teleport");
        ui.label("Select a map id and hit ▶ and you will be teleported to it.");
        ui.horizontal(|ui| {
            // let keys: Vec<_> = self.maps.keys().collect();
            let name = self.maps.get(&self.selected_map).map_or("", String::as_str);
            ui.add(
                DragValue::new(&mut self.selected_map.0)
                    .suffix(format!(". {}", name))
            );
            // auto_complete(ui, ui.id().with("map warp"), &keys, &mut self.selected_map);

            // fn label_text(id: MapHash, name: Option<impl AsRef<str>>) -> impl Into<WidgetText> {
            //     if let Some(name) = name {
            //         RichText::new(format!("{}. {}", id.0, name.as_ref()))
            //     } else {
            //         RichText::new(format!("{}. new map", id.0)).italics()
            //     }
            // }
            // let selected_text = label_text(self.selected_id, self.maps.get(&self.selected_id));
            // egui::ComboBox::from_id_source("map selecter")
            //     .selected_text(selected_text)
            //     .show_ui(ui, |ui| {
            //         ui.set_min_width(200.0);
            //         let max_id = self.maps.keys().fold(0, |acc, k| k.0.max(acc)) + 1;
            //         for id in 0..=max_id {
            //             let key = MapHash(id);
            //             let label = label_text(key, self.maps.get(&key));
            //             if ui.selectable_label(self.selected_id == key, label).clicked() {
            //                 self.selected_id = key;
            //             }
            //         }
            //     });

            if ui.button("▶").clicked() {
                let map = std::mem::take(&mut self.selected_map);
                self.wants = Some(Wants::Warp(map));
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
            texture: IVec2::new(
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
