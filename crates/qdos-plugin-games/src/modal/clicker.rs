//! Clicker game modal rendering
//!
//! This module handles the drawing of the Clicker roguelike game UI including
//! the dungeon corridor, player/monster stats, shop, death screen, and soul shop.

use super::super::clicker::{
    Buff, ClickerState, ClickerView, Item, Scenery, ShopItem, SoulUpgrade,
};
use qdos_plugin_api::ui::FullScreenView;
use qdos_plugin_api::ThemeColors;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

// =============================================================================
// CLICKER GAME STRINGS & ASCII ART
// =============================================================================

/// Death screen title
pub const CLICKER_DEATH_TITLE: &str = "X  Y O U   D I E D  X";

/// Castle silhouette - dark and foreboding
pub const CASTLE_ART: &[&str] = &[
    "                  │▌              │▌              │▌                  ",
    "       ▄█▄       ███▄            ███▄            ███▄       ▄█▄       ",
    "      █████     █████           █████           █████     █████      ",
    "     ███████   ███████    ▄    ███████    ▄    ███████   ███████     ",
    "    ▄███████▄▄█████████▄▄███▄▄█████████▄▄███▄▄█████████▄▄███████▄    ",
    "   ██████████████████████████████████████████████████████████████   ",
    "   ██▓▓██░░██▓▓██░░██▓▓██░░██▓▓██░░██▓▓██░░██▓▓██░░██▓▓██░░██▓▓██   ",
    "   ██████████████████████████████████████████████████████████████   ",
    "   ██  ██  ██  ██  ██  ████████████████████  ██  ██  ██  ██  ██   ",
    "   ██  ██  ██  ██  ██  ██▓▓▓▓▓▓▓▓▓▓▓▓▓▓██  ██  ██  ██  ██  ██   ",
    "   ██████████████████████              ██████████████████████████   ",
];

/// Flag animation frames (waving)
pub const FLAG_WAVE_1: &str = "▀▄";
pub const FLAG_WAVE_2: &str = "▄▀";
pub const FLAG_WAVE_3: &str = "▀▄";
pub const FLAG_WAVE_4: &str = " ▀";

/// Skull with glowing eyes
pub const SKULL_SMALL: &[&str] = &[
    "    ▄▄███▄▄    ",
    "   ███●█●███   ",
    "   ██▄███▄██   ",
    "    ▀█▀▀▀█▀    ",
];

/// Rogue with knife ASCII art - fallen hero
pub const ROGUE_FALLEN: &[&str] = &["      ╪═─  O   ", "        \\ /|\\  ", "          / \\ "];

/// Soul shop title
pub const CLICKER_SOUL_SHOP_TITLE: &str = "~ S O U L   S H O P ~";

/// Elite monster prefix
pub const CLICKER_ELITE_PREFIX: &str = "* ";

/// Floor boss prefix
pub const CLICKER_FLOOR_BOSS_PREFIX: &str = "+ FLOOR BOSS + ";

/// Boss prefix
pub const CLICKER_BOSS_PREFIX: &str = "* ";

// =============================================================================
// CLICKER DRAWING FUNCTIONS
// =============================================================================

/// Main clicker drawing dispatcher - routes to appropriate view
pub fn draw_clicker(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ClickerState,
    colors: &ThemeColors,
) {
    // Dispatch based on ClickerView
    match state.view {
        ClickerView::Playing => draw_clicker_playing(frame, view, state, colors),
        ClickerView::Dead => draw_clicker_dead(frame, view, state, colors),
        ClickerView::SoulShop => draw_clicker_soul_shop(frame, view, state, colors),
    }
}

/// Draws the death screen with castle, skull, fallen rogue, and run statistics
pub fn draw_clicker_dead(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ClickerState,
    colors: &ThemeColors,
) {
    let mut row = 0u16;

    // === DARK SKY WITH BLOOD RED GRADIENT ===
    view.render_row(
        frame,
        row,
        vec![Span::styled(
            "░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░",
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::DIM),
        )],
    );
    row += 1;

    // === CASTLE WITH ANIMATED FLAGS ===
    // Flag animation based on tick
    let flag_frame = (state.tick / 3) % 4;
    let _flag = match flag_frame {
        0 => FLAG_WAVE_1,
        1 => FLAG_WAVE_2,
        2 => FLAG_WAVE_3,
        _ => FLAG_WAVE_4,
    };

    // Draw castle with colored elements and animated flag color
    let flag_color = if flag_frame.is_multiple_of(2) {
        colors.red()
    } else {
        colors.yellow()
    };

    for (i, line) in CASTLE_ART.iter().enumerate() {
        let mut spans: Vec<Span> = Vec::new();

        for ch in line.chars() {
            let (color, modifier) = match ch {
                // Flag poles and flags - animated colors
                '│' | '▌' => {
                    if i == 0 {
                        // Flag pole top - use animated color
                        (flag_color, Modifier::BOLD)
                    } else {
                        (colors.grey(), Modifier::empty())
                    }
                }
                // Castle structure
                '█' | '▄' => (colors.grey(), Modifier::DIM),
                '▓' => (colors.yellow(), Modifier::DIM), // Lit windows
                '░' => (colors.grey(), Modifier::DIM),   // Darker stone
                _ => (colors.grey(), Modifier::DIM),
            };
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(color).add_modifier(modifier),
            ));
        }
        view.render_row(frame, row, spans);
        row += 1;
    }

    // === SKULL IN CENTER ===
    for line in SKULL_SMALL.iter() {
        let mut spans: Vec<Span> = Vec::new();
        for ch in line.chars() {
            let color = match ch {
                '●' => colors.red(), // Glowing red eyes
                '█' | '▄' | '▀' => colors.fg(),
                _ => colors.grey(),
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
        }
        view.render_row(frame, row, spans);
        row += 1;
    }

    // === FALLEN ROGUE ===
    for line in ROGUE_FALLEN.iter() {
        let mut spans: Vec<Span> = Vec::new();
        for ch in line.chars() {
            let color = match ch {
                'O' => colors.yellow(),
                '/' | '\\' | '|' => colors.cyan(),
                '─' | '═' | '╪' => colors.fg(),
                _ => colors.grey(),
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
        }
        view.render_row(frame, row, spans);
        row += 1;
    }

    let center_row = row + 1;

    // Death title
    view.render_row(
        frame,
        center_row - 1,
        vec![Span::styled(
            "╔══════════════════════════════════════════════════╗",
            Style::default().fg(colors.red()),
        )],
    );
    view.render_row(
        frame,
        center_row,
        vec![Span::styled(
            format!("║{:^50}║", CLICKER_DEATH_TITLE),
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        center_row + 1,
        vec![Span::styled(
            "╠══════════════════════════════════════════════════╣",
            Style::default().fg(colors.red()),
        )],
    );

    // Run stats
    view.render_row(
        frame,
        center_row + 2,
        vec![Span::styled(
            format!("║  {:.<23} {:>22}  ║", "Floor Reached", state.dungeon_floor),
            Style::default().fg(colors.fg()),
        )],
    );
    view.render_row(
        frame,
        center_row + 3,
        vec![Span::styled(
            format!("║  {:.<23} {:>22}  ║", "Level Reached", state.level),
            Style::default().fg(colors.fg()),
        )],
    );
    view.render_row(
        frame,
        center_row + 4,
        vec![Span::styled(
            format!(
                "║  {:.<23} {:>22}  ║",
                "Monsters Slain", state.monsters_killed
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    view.render_row(
        frame,
        center_row + 5,
        vec![Span::styled(
            format!(
                "║  {:.<23} {:>22}  ║",
                "Bosses Defeated", state.bosses_killed
            ),
            Style::default().fg(colors.fg()),
        )],
    );
    view.render_row(
        frame,
        center_row + 6,
        vec![Span::styled(
            format!(
                "║  {:.<23} {:>22}  ║",
                "Gold Earned", state.total_gold_earned
            ),
            Style::default().fg(colors.yellow()),
        )],
    );

    // Souls earned - the big reward!
    view.render_row(
        frame,
        center_row + 7,
        vec![Span::styled(
            format!("║{:^50}║", "────────────────────────────"),
            Style::default().fg(colors.red()),
        )],
    );
    view.render_row(
        frame,
        center_row + 8,
        vec![Span::styled(
            format!(
                "║{:^50}║",
                format!("+ SOULS EARNED: {} +", state.souls.souls_earned_this_run)
            ),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        center_row + 9,
        vec![Span::styled(
            format!(
                "║{:^50}║",
                format!("Total Souls: {}", state.souls.total_souls)
            ),
            Style::default().fg(colors.cyan()),
        )],
    );

    view.render_row(
        frame,
        center_row + 10,
        vec![Span::styled(
            "╚══════════════════════════════════════════════════╝",
            Style::default().fg(colors.red()),
        )],
    );

    let help = vec![
        ("Enter/r", "new run"),
        ("s/Tab", "soul shop"),
        ("Esc", "menu"),
    ];
    view.render_help(frame, help);
}

/// Draws the soul shop for spending souls on permanent upgrades
pub fn draw_clicker_soul_shop(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ClickerState,
    colors: &ThemeColors,
) {
    // Header
    view.render_row(
        frame,
        1,
        vec![Span::styled(
            "╔════════════════════════════════════════════════════════════════════════╗",
            Style::default().fg(colors.cyan()),
        )],
    );
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            format!("║{:^72}║", CLICKER_SOUL_SHOP_TITLE),
            Style::default()
                .fg(colors.cyan())
                .add_modifier(Modifier::BOLD),
        )],
    );
    view.render_row(
        frame,
        3,
        vec![Span::styled(
            format!("║{:^72}║", format!("Souls: {}", state.souls.total_souls)),
            Style::default().fg(colors.yellow()),
        )],
    );
    view.render_row(
        frame,
        4,
        vec![Span::styled(
            "╠════════════════════════════════════════════════════════════════════════╣",
            Style::default().fg(colors.cyan()),
        )],
    );

    // Upgrades list
    for (i, upgrade) in SoulUpgrade::all().iter().enumerate() {
        let is_selected = i == state.soul_shop_selected;
        let current_level = state.souls.upgrade_level(*upgrade);
        let max_level = upgrade.max_level();
        let cost = state.souls.upgrade_cost(*upgrade);
        let can_afford = state.souls.can_afford(*upgrade);
        let is_maxed = current_level >= max_level;

        let prefix = if is_selected { "►" } else { " " };
        let level_str = if is_maxed {
            "MAX".to_string()
        } else {
            format!("Lv.{}", current_level)
        };
        let cost_str = if is_maxed {
            "---".to_string()
        } else {
            format!("{} souls", cost)
        };

        let style = if is_maxed {
            Style::default().fg(colors.grey())
        } else if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else if can_afford {
            Style::default().fg(colors.green())
        } else {
            Style::default().fg(colors.grey())
        };

        let line = format!(
            "║ {} {:<18} {:>6}  {:<30} {:>10} ║",
            prefix,
            upgrade.name(),
            level_str,
            upgrade.description(),
            cost_str
        );
        view.render_row(frame, 5 + i as u16, vec![Span::styled(line, style)]);
    }

    // Bottom border
    let footer_row = 5 + SoulUpgrade::all().len() as u16;
    view.render_row(
        frame,
        footer_row,
        vec![Span::styled(
            "╚════════════════════════════════════════════════════════════════════════╝",
            Style::default().fg(colors.cyan()),
        )],
    );

    // Soul bonuses summary
    view.render_row(
        frame,
        footer_row + 2,
        vec![
            Span::styled("Current Bonuses: ", Style::default().fg(colors.grey())),
            Span::styled(
                format!("STR+{} ", state.souls.starting_str),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!("ARM+{} ", state.souls.starting_arm),
                Style::default().fg(colors.blue()),
            ),
            Span::styled(
                format!("HP+{} ", state.souls.starting_hp * 10),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                format!("Gold+{} ", state.souls.starting_gold * 50),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                format!("Speed+{}% ", state.souls.attack_speed * 10),
                Style::default().fg(colors.red()),
            ),
        ],
    );

    view.render_row(
        frame,
        footer_row + 3,
        vec![
            Span::styled("               ", Style::default()),
            Span::styled(
                format!("Crit×{:.1} ", state.souls.crit_damage_multiplier()),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                format!("Gold+{}% ", state.souls.soul_gold_multiplier()),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                format!("Drop+{}% ", state.souls.soul_drop_bonus()),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!("Floor+{}", state.souls.floor_skip),
                Style::default().fg(colors.blue()),
            ),
        ],
    );

    let help = if state.game_over {
        vec![
            ("↑↓", "select"),
            ("Enter/b", "buy"),
            ("r", "new run"),
            ("X", "reset ALL"),
            ("Esc", "back"),
        ]
    } else {
        vec![
            ("↑↓", "select"),
            ("Enter/b", "buy"),
            ("X", "reset ALL"),
            ("Esc", "back"),
        ]
    };
    view.render_help(frame, help);
}

/// Draws the main playing view with dungeon corridor, stats, shop, and inventory
pub fn draw_clicker_playing(
    frame: &mut Frame,
    view: &FullScreenView,
    state: &ClickerState,
    colors: &ThemeColors,
) {
    // Helper to get scenery color
    let scenery_color = |s: &Scenery| match s.color_idx {
        1 => colors.yellow(),
        2 => colors.cyan(),
        3 => colors.red(),
        4 => colors.green(),
        _ => colors.grey(),
    };

    // Calculate layout - corridor on left, shop on right
    let content_width = view.area.width as usize;
    let shop_width = 26;
    let corridor_width = content_width.saturating_sub(shop_width + 2);

    // === TOP STATUS BAR ===
    let total_str = state.total_strength();
    let total_arm = state.total_armor();

    // Calculate STR buff amount for display
    let str_buff: i32 = state
        .buffs
        .iter()
        .map(|b| match b {
            Buff::Strength(amt, _) => *amt,
            _ => 0,
        })
        .sum();

    // Calculate ARM gear bonus for display
    let base_arm = state.armor + state.armor_equip.as_ref().map_or(0, |a| a.bonus);
    let arm_gear_bonus = total_arm - base_arm;

    // First row: main stats
    view.render_row(
        frame,
        0,
        vec![
            Span::styled(
                format!("HP:{}/{}", state.hp, state.max_hp),
                Style::default().fg(if state.hp < state.max_hp / 3 {
                    colors.red()
                } else {
                    colors.green()
                }),
            ),
            Span::styled(
                if str_buff > 0 {
                    format!(" STR:{}+{}", total_str - str_buff, str_buff)
                } else {
                    format!(" STR:{}", total_str)
                },
                Style::default().fg(if str_buff > 0 {
                    colors.yellow() // Highlight when buffed
                } else {
                    colors.cyan()
                }),
            ),
            Span::styled(
                if arm_gear_bonus > 0 {
                    format!(" ARM:{}+{}", base_arm, arm_gear_bonus)
                } else {
                    format!(" ARM:{}", total_arm)
                },
                Style::default().fg(if arm_gear_bonus > 0 {
                    colors.cyan() // Highlight when has gear bonuses
                } else {
                    colors.blue()
                }),
            ),
            Span::styled(
                format!(" Gold:{}", state.gold),
                Style::default().fg(colors.yellow()),
            ),
            Span::styled(
                format!(" Lv:{}", state.level),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!(" XP:{}/{}", state.xp, state.xp_for_level()),
                Style::default().fg(colors.green()),
            ),
            Span::styled(
                format!(" Food:{}", state.food),
                Style::default().fg(if state.food < 5 {
                    colors.red()
                } else {
                    colors.fg()
                }),
            ),
        ],
    );

    // Second row: biome, floor, class, alchemy, souls, dust
    let class_name = state.souls.selected_class.name();
    let alchemy_tier = state.alchemy_tier();
    let biome_name = state.biome.name();

    view.render_row(
        frame,
        1,
        vec![
            Span::styled(
                format!("Floor:{}", state.dungeon_floor),
                Style::default().fg(colors.cyan()),
            ),
            Span::styled(
                format!(" {}", biome_name),
                Style::default()
                    .fg(match state.biome.color_idx() {
                        3 => colors.red(),
                        4 => colors.green(),
                        5 => colors.blue(),
                        _ => colors.grey(),
                    })
                    .add_modifier(Modifier::DIM),
            ),
            if class_name != "Peasant" {
                Span::styled(
                    format!(" [{}]", class_name),
                    Style::default().fg(colors.yellow()),
                )
            } else {
                Span::styled("", Style::default())
            },
            if state.souls.alchemy_level > 0 {
                Span::styled(
                    format!(" Alch:{}", alchemy_tier.name()),
                    Style::default().fg(colors.green()),
                )
            } else {
                Span::styled("", Style::default())
            },
            if state.souls.total_souls > 0 {
                Span::styled(
                    format!(" Souls:{}", state.souls.total_souls),
                    Style::default().fg(colors.cyan()),
                )
            } else {
                Span::styled("", Style::default())
            },
            if state.souls.dust > 0 {
                Span::styled(
                    format!(" Dust:{}", state.souls.dust),
                    Style::default().fg(colors.blue()),
                )
            } else {
                Span::styled("", Style::default())
            },
            // Monster Zoo event indicator
            if state.zoo_event.active {
                Span::styled(
                    format!(
                        " ZOO! {}left {:0.1}s",
                        state.zoo_event.monsters_remaining,
                        state.zoo_event.time_remaining as f32 / 20.0
                    ),
                    Style::default()
                        .fg(colors.red())
                        .add_modifier(Modifier::BOLD | Modifier::RAPID_BLINK),
                )
            } else {
                Span::styled("", Style::default())
            },
        ],
    );

    // === DUNGEON CORRIDOR (full width minus shop) ===
    let corridor_border = "═".repeat(corridor_width.saturating_sub(2));
    view.render_row(
        frame,
        2,
        vec![Span::styled(
            format!("╔{}╗", corridor_border),
            Style::default().fg(colors.grey()),
        )],
    );

    // Floor with colored scenery, player, and monster
    let mut floor_spans: Vec<Span> = vec![Span::styled("║", Style::default().fg(colors.grey()))];

    let player_pos = 8;
    let monster_pos = corridor_width / 2;

    for (i, scenery) in state
        .floor
        .iter()
        .take(corridor_width.saturating_sub(2))
        .enumerate()
    {
        if i == player_pos {
            floor_spans.push(Span::styled("@", Style::default().fg(colors.yellow())));
        } else if i == monster_pos {
            if let Some(ref monster) = state.current_monster {
                let monster_style = if monster.is_boss {
                    Style::default()
                        .fg(colors.red())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors.red())
                };
                floor_spans.push(Span::styled(monster.char.to_string(), monster_style));
            } else {
                floor_spans.push(Span::styled(
                    scenery.char.to_string(),
                    Style::default().fg(scenery_color(scenery)),
                ));
            }
        } else {
            floor_spans.push(Span::styled(
                scenery.char.to_string(),
                Style::default().fg(scenery_color(scenery)),
            ));
        }
    }
    floor_spans.push(Span::styled("║", Style::default().fg(colors.grey())));

    view.render_row(frame, 3, floor_spans);

    view.render_row(
        frame,
        4,
        vec![Span::styled(
            format!("╚{}╝", corridor_border),
            Style::default().fg(colors.grey()),
        )],
    );

    // === MONSTER INFO ===
    if let Some(ref monster) = state.current_monster {
        // Determine prefix based on monster type (uses constants for easy editing)
        let prefix = if monster.is_floor_boss {
            CLICKER_FLOOR_BOSS_PREFIX
        } else if monster.is_boss {
            CLICKER_BOSS_PREFIX
        } else if !monster.affixes.is_empty() {
            CLICKER_ELITE_PREFIX // Elite indicator
        } else {
            ""
        };

        // Determine color based on monster type
        let name_color = if monster.is_floor_boss {
            colors.red()
        } else if monster.is_boss {
            colors.red()
        } else if !monster.affixes.is_empty() {
            colors.yellow() // Elite = yellow
        } else {
            colors.fg()
        };

        view.render_row(
            frame,
            6,
            vec![
                Span::styled(
                    format!("{}Enemy: {} ", prefix, monster.name),
                    Style::default().fg(name_color).add_modifier(
                        if monster.is_boss || !monster.affixes.is_empty() {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        },
                    ),
                ),
                Span::styled(
                    format!("HP:{}/{}", monster.hp.max(0), monster.max_hp),
                    Style::default().fg(if monster.hp > monster.max_hp / 2 {
                        colors.green()
                    } else if monster.hp > 0 {
                        colors.yellow()
                    } else {
                        colors.red()
                    }),
                ),
                Span::styled(
                    format!("  Next boss: {} kills", state.kills_until_boss),
                    Style::default().fg(colors.grey()),
                ),
            ],
        );

        view.render_row(
            frame,
            7,
            vec![Span::styled(
                monster.description.clone(),
                Style::default().fg(colors.grey()),
            )],
        );

        // HP bar
        let bar_width = 30;
        let hp_pct = (monster.hp.max(0) as f32 / monster.max_hp as f32 * bar_width as f32) as usize;
        let hp_bar = "█".repeat(hp_pct) + &"░".repeat(bar_width - hp_pct);
        view.render_row(
            frame,
            8,
            vec![Span::styled(
                format!("[{}]", hp_bar),
                Style::default().fg(if monster.is_boss {
                    colors.red()
                } else {
                    colors.cyan()
                }),
            )],
        );
    }

    // === MESSAGE AREA ===
    if let Some(ref msg) = state.message {
        let msg_style = if state.last_crit {
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.yellow())
        };
        view.render_row(frame, 10, vec![Span::styled(msg.clone(), msg_style)]);
    }

    // === EQUIPMENT & STATS ===
    let weapon_str = state
        .weapon
        .as_ref()
        .map_or("None".to_string(), |w| w.name.clone());
    let armor_str = state
        .armor_equip
        .as_ref()
        .map_or("None".to_string(), |a| a.name.clone());

    view.render_row(
        frame,
        12,
        vec![
            Span::styled("Weapon: ", Style::default().fg(colors.grey())),
            Span::styled(weapon_str, Style::default().fg(colors.cyan())),
            Span::styled("  Armor: ", Style::default().fg(colors.grey())),
            Span::styled(armor_str, Style::default().fg(colors.blue())),
        ],
    );

    // New gear slots (helm, amulet, cloak, gloves, boots, shield)
    let helm_str = state
        .helm
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());
    let amulet_str = state
        .amulet
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());
    let cloak_str = state
        .cloak
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());
    let gloves_str = state
        .gloves
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());
    let boots_str = state
        .boots
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());
    let shield_str = state
        .shield
        .as_ref()
        .map_or("-".to_string(), |g| g.name.clone());

    // Only show if player has any gear equipped
    let has_gear = state.helm.is_some()
        || state.amulet.is_some()
        || state.cloak.is_some()
        || state.gloves.is_some()
        || state.boots.is_some()
        || state.shield.is_some();

    if has_gear {
        // Truncate names to fit
        let truncate = |s: String| -> String {
            if s.len() > 12 {
                format!("{}...", &s[..9])
            } else {
                s
            }
        };
        view.render_row(
            frame,
            18,
            vec![
                Span::styled("Gear: ", Style::default().fg(colors.grey())),
                Span::styled(
                    format!("H:{} ", truncate(helm_str)),
                    Style::default().fg(colors.cyan()),
                ),
                Span::styled(
                    format!("A:{} ", truncate(amulet_str)),
                    Style::default().fg(colors.yellow()),
                ),
                Span::styled(
                    format!("C:{} ", truncate(cloak_str)),
                    Style::default().fg(colors.blue()),
                ),
            ],
        );
        view.render_row(
            frame,
            19,
            vec![
                Span::styled("      ", Style::default()),
                Span::styled(
                    format!("G:{} ", truncate(gloves_str)),
                    Style::default().fg(colors.red()),
                ),
                Span::styled(
                    format!("B:{} ", truncate(boots_str)),
                    Style::default().fg(colors.green()),
                ),
                Span::styled(
                    format!("S:{}", truncate(shield_str)),
                    Style::default().fg(colors.cyan()),
                ),
            ],
        );
    }

    // Status indicators - Floor, Level, Kills
    let stairs_indicator = if state.stairs_available { " [%]" } else { "" };
    let mut status_spans = vec![
        Span::styled(
            format!("Floor:{}", state.dungeon_floor),
            Style::default().fg(colors.blue()),
        ),
        Span::styled(
            format!("  Lv:{}", state.dungeon_level),
            Style::default().fg(colors.cyan()),
        ),
        Span::styled(
            format!("  Kills:{}", state.monsters_killed),
            Style::default().fg(colors.grey()),
        ),
    ];
    if state.stairs_available {
        status_spans.push(Span::styled(
            stairs_indicator,
            Style::default()
                .fg(colors.yellow())
                .add_modifier(Modifier::BOLD),
        ));
    }
    view.render_row(frame, 13, status_spans);

    // Rings and active buffs
    let mut ring_spans = vec![Span::styled("Rings: ", Style::default().fg(colors.grey()))];
    for (i, ring) in state.ring_slots.iter().enumerate() {
        if i > 0 {
            ring_spans.push(Span::styled(" ", Style::default()));
        }
        match ring {
            Some(r) => ring_spans.push(Span::styled(
                format!("={}", r.name()),
                Style::default().fg(colors.cyan()),
            )),
            None => ring_spans.push(Span::styled("=none", Style::default().fg(colors.grey()))),
        }
    }
    // Add active buffs
    if !state.buffs.is_empty() {
        ring_spans.push(Span::styled("  Buffs:", Style::default().fg(colors.grey())));
        for buff in &state.buffs {
            let buff_str = match buff {
                Buff::Strength(amt, _) => format!(" STR+{}", amt),
                Buff::Speed(_) => " FAST".to_string(),
                Buff::GoldRush(k) => format!(" GOLD({})", k),
                Buff::IceSlow(_) => " ICE".to_string(),
            };
            ring_spans.push(Span::styled(
                buff_str,
                Style::default()
                    .fg(colors.yellow())
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }
    view.render_row(frame, 14, ring_spans);

    // Inventory display
    let mut inv_spans = vec![Span::styled("Pack: ", Style::default().fg(colors.grey()))];
    for (i, item) in state.inventory.iter().enumerate() {
        let is_selected = i == state.inv_selected;
        let item_char = item.char();
        let style = if is_selected {
            Style::default().fg(colors.yellow()).bg(colors.red())
        } else {
            match item {
                Item::Potion(_) => Style::default().fg(colors.green()),
                Item::Scroll(_) => Style::default().fg(colors.cyan()),
                Item::Ring(_) => Style::default().fg(colors.blue()),
                Item::Wand(_, _) => Style::default().fg(colors.yellow()),
            }
        };
        inv_spans.push(Span::styled(format!("{}{}", i + 1, item_char), style));
        inv_spans.push(Span::styled(" ", Style::default()));
    }
    // Show empty slots
    for i in state.inventory.len()..8 {
        inv_spans.push(Span::styled(
            format!("{}.", i + 1),
            Style::default().fg(colors.grey()),
        ));
        inv_spans.push(Span::styled(" ", Style::default()));
    }
    view.render_row(frame, 15, inv_spans);

    // Selected item description
    if !state.inventory.is_empty() && state.inv_selected < state.inventory.len() {
        let sel_item = &state.inventory[state.inv_selected];
        view.render_row(
            frame,
            16,
            vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{}: {}", sel_item.name(), sel_item.description()),
                    Style::default().fg(colors.grey()),
                ),
            ],
        );
    }

    // Auto modes
    let mut auto_spans = Vec::new();
    if state.auto_attack {
        auto_spans.push(Span::styled(
            "[AUTO-HIT]",
            Style::default().fg(colors.green()),
        ));
    }
    if state.auto_eat {
        auto_spans.push(Span::styled(
            format!(" [AUTO-EAT@{}%]", state.auto_eat_threshold),
            Style::default().fg(colors.cyan()),
        ));
    }
    if state.auto_quaff {
        auto_spans.push(Span::styled(
            " [AUTO-QUAFF]",
            Style::default().fg(colors.yellow()),
        ));
    }
    if state.auto_equip {
        auto_spans.push(Span::styled(
            " [AUTO-EQUIP]",
            Style::default().fg(colors.blue()),
        ));
    }
    // Show combat lanes if > 1
    if state.combat_lanes > 1 {
        auto_spans.push(Span::styled(
            format!(" [LANES:{}]", state.combat_lanes),
            Style::default()
                .fg(colors.red())
                .add_modifier(Modifier::BOLD),
        ));
    }
    if !auto_spans.is_empty() {
        view.render_row(frame, 17, auto_spans);
    }

    // === SHOP (always visible on right side) ===
    let shop_x = (corridor_width + 2) as u16;
    let content_y = view.content_start_y();
    let shop_header = format!("{:^24}", "═══ SHOP ═══");
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            shop_header,
            Style::default().fg(colors.yellow()),
        )])),
        Rect::new(shop_x, content_y + 1, shop_width as u16, 1),
    );

    // Shop items
    for (i, item) in ShopItem::all().iter().enumerate() {
        let is_selected = i == state.shop_selected;
        let cost = state.item_cost(*item);
        let can_afford = state.can_afford(*item);
        let is_maxed = state.is_maxed(*item);

        let prefix = if is_selected { "►" } else { " " };

        let style = if is_maxed {
            Style::default().fg(colors.grey())
        } else if is_selected {
            Style::default()
                .fg(colors.yellow())
                .bg(colors.red())
                .add_modifier(Modifier::BOLD)
        } else if can_afford {
            Style::default().fg(colors.green())
        } else {
            Style::default().fg(colors.grey())
        };

        let cost_str = if is_maxed {
            "MAX".to_string()
        } else {
            format!("{}g", cost)
        };

        let item_line = format!("{}{:<14}{:>5}", prefix, item.name(), cost_str);
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(item_line, style)])),
            Rect::new(shop_x, content_y + 2 + i as u16, shop_width as u16, 1),
        );
    }

    // Shop footer with description
    let selected_item = state.selected_shop_item();
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            selected_item.description(),
            Style::default().fg(colors.grey()),
        )])),
        Rect::new(
            shop_x,
            content_y + 2 + ShopItem::all().len() as u16 + 1,
            shop_width as u16,
            1,
        ),
    );

    let help = vec![
        ("Space", "hit"),
        ("e", "eat"),
        ("1-8/q", "use"),
        (">", "stairs"),
        ("b", "buy"),
        ("s", "souls"),
        ("Esc", "quit"),
    ];
    view.render_help(frame, help);
}
