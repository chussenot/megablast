//! HUD: HP bar (bottom-left), cash (bottom-right), score (top-left),
//! level (top-center); the shop screen.
//!
//! Plumbing (this doc comment's owner: bootstrap/integrator) -- the
//! glyphon text pipeline (`FontSystem`/`TextRenderer`/etc.) and the quad
//! pipeline both live in `render/mod.rs`, which you don't own; `build`
//! below is the seam: return the quads/text you want drawn, in logical
//! playfield coordinates, and `Renderer::render` handles turning them
//! into an actual draw call. Use `super::make_line_buffer(font_system,
//! text, font_size, line_height, wrap_width, align)` to shape a line of
//! text (mirrors arkanoid's identical helper), and `super::QuadInstance`
//! (accessible as a private sibling item since this module is a
//! descendant of `render`) for a quad -- e.g. an HP bar's background and
//! fill.
//!
//! Owner: Wave 3 `hud-lives` (HP/cash/score/level). Extended by Wave 4
//! `shop-wiring` (shop screen), Wave 5 `juice-fx` (HP-bar flash on
//! damage).

use glyphon::cosmic_text::Align;
use glyphon::{Buffer as TextBuffer, Color as TextColor, FontSystem};

use crate::game::{player, shop, weapons, Game, GameState, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};

/// Headroom reserved in `mod.rs`'s shared instance buffer for this
/// module's own quads (e.g. an HP bar's background + fill) -- bump if a
/// future wave's worst-case frame needs more than it reserves.
pub(super) const MAX_HUD_QUADS: usize = 8;

/// One piece of HUD text: a shaped glyphon buffer plus where to draw it,
/// in the same logical playfield coordinates as everything else
/// (`Renderer::render` converts to physical pixels for you via
/// `to_physical`).
pub(super) struct HudText {
    pub buffer: TextBuffer,
    pub x: f32,
    pub y: f32,
    pub color: TextColor,
}

/// Everything this module wants drawn this frame.
pub(super) struct HudDraw {
    pub quads: Vec<super::QuadInstance>,
    pub texts: Vec<HudText>,
}

/// Shared margin from the playfield edges for every HUD element.
const HUD_MARGIN: f32 = 10.0;
const HUD_FONT_SIZE: f32 = 20.0;
const HUD_LINE_HEIGHT: f32 = 24.0;
const HUD_TEXT_COLOR: TextColor = TextColor::rgb(230, 232, 240);

const HP_BAR_WIDTH: f32 = 150.0;
const HP_BAR_HEIGHT: f32 = 16.0;
const HP_BAR_BG_COLOR: [f32; 4] = [0.25, 0.05, 0.05, 1.0];
const HP_BAR_FILL_COLOR: [f32; 4] = [0.2, 0.85, 0.25, 1.0];

/// Shop screen (Wave 4 `shop-wiring`). Six purchasable items (spec
/// order, matching `shop::Item`) plus a 7th "Leave" sentinel slot --
/// same slot count and order as `game::mod::tick_shop`'s `SHOP_ITEMS`;
/// the two files must stay in sync if this ever changes.
const SHOP_ITEM_COUNT: usize = 6;
const SHOP_SLOT_COUNT: usize = SHOP_ITEM_COUNT + 1;
const SHOP_LEAVE_SLOT: usize = SHOP_ITEM_COUNT;
const SHOP_TITLE_Y: f32 = 220.0;
const SHOP_ITEM_START_Y: f32 = 264.0;
const SHOP_ITEM_LINE_HEIGHT: f32 = 30.0;
const SHOP_ITEM_COLOR: TextColor = TextColor::rgb(190, 195, 210);
const SHOP_HIGHLIGHT_COLOR: TextColor = TextColor::rgb(255, 215, 80);

/// One shop menu line's label: item name, and price or owned-state
/// (spec: fixed prices, tier/count for Cannon/Drone, OWNED for the
/// flat side/rear shots). Slot `SHOP_LEAVE_SLOT` has no price.
fn shop_item_label(loadout: &weapons::Loadout, slot: usize) -> String {
    match slot {
        0 if loadout.cannon_tier as usize > shop::CANNON_PRICES.len() => {
            format!("CANNON T{}  MAX", loadout.cannon_tier)
        }
        0 => format!(
            "CANNON T{}  {}",
            loadout.cannon_tier,
            shop::CANNON_PRICES[loadout.cannon_tier as usize - 1]
        ),
        1 if loadout.has_side => "SIDE SHOTS  OWNED".to_string(),
        1 => format!("SIDE SHOTS  {}", shop::SIDE_SHOTS_PRICE),
        2 if loadout.has_rear => "REAR SHOT  OWNED".to_string(),
        2 => format!("REAR SHOT  {}", shop::REAR_SHOT_PRICE),
        3 if loadout.drones >= shop::MAX_DRONES => {
            format!("DRONE {}/{}  MAX", loadout.drones, shop::MAX_DRONES)
        }
        3 => format!(
            "DRONE {}/{}  {}",
            loadout.drones,
            shop::MAX_DRONES,
            shop::DRONE_PRICE
        ),
        4 => format!(
            "REPAIR {}%  {}",
            (shop::REPAIR_PERCENT * 100.0) as u32,
            shop::REPAIR_PRICE
        ),
        5 => format!("EXTRA LIFE  {}", shop::EXTRA_LIFE_PRICE),
        _ => {
            debug_assert_eq!(slot, SHOP_LEAVE_SLOT, "unexpected shop slot {slot}");
            "LEAVE - NEXT LEVEL".to_string()
        }
    }
}

/// HP bar bottom-left (background + fill scaled to `game.player.hp /
/// player::MAX_HP`), cash bottom-right, score top-left, level top-center
/// -- all in logical playfield coordinates (spec section "HUD").
pub(super) fn build(font_system: &mut FontSystem, game: &Game) -> HudDraw {
    let bar_x = HUD_MARGIN;
    let bar_y = PLAYFIELD_HEIGHT - HUD_MARGIN - HP_BAR_HEIGHT;
    let hp_frac = (game.player.hp / player::MAX_HP).clamp(0.0, 1.0);
    let fill_width = HP_BAR_WIDTH * hp_frac;

    let quads = vec![
        super::QuadInstance::new(
            [bar_x + HP_BAR_WIDTH / 2.0, bar_y + HP_BAR_HEIGHT / 2.0],
            [HP_BAR_WIDTH / 2.0, HP_BAR_HEIGHT / 2.0],
            HP_BAR_BG_COLOR,
        ),
        super::QuadInstance::new(
            [bar_x + fill_width / 2.0, bar_y + HP_BAR_HEIGHT / 2.0],
            [fill_width / 2.0, HP_BAR_HEIGHT / 2.0],
            HP_BAR_FILL_COLOR,
        ),
    ];
    debug_assert!(quads.len() <= MAX_HUD_QUADS);

    let score_buffer = super::make_line_buffer(
        font_system,
        &format!("SCORE {}", game.score),
        HUD_FONT_SIZE,
        HUD_LINE_HEIGHT,
        PLAYFIELD_WIDTH,
        Align::Left,
    );
    let level_buffer = super::make_line_buffer(
        font_system,
        &format!("LEVEL {}", game.level),
        HUD_FONT_SIZE,
        HUD_LINE_HEIGHT,
        PLAYFIELD_WIDTH,
        Align::Center,
    );
    let cash_buffer = super::make_line_buffer(
        font_system,
        &format!("CASH {}", game.shop.cash),
        HUD_FONT_SIZE,
        HUD_LINE_HEIGHT,
        PLAYFIELD_WIDTH - HUD_MARGIN,
        Align::Right,
    );

    let mut texts = vec![
        HudText {
            buffer: score_buffer,
            x: HUD_MARGIN,
            y: HUD_MARGIN,
            color: HUD_TEXT_COLOR,
        },
        HudText {
            buffer: level_buffer,
            x: 0.0,
            y: HUD_MARGIN,
            color: HUD_TEXT_COLOR,
        },
        HudText {
            buffer: cash_buffer,
            x: 0.0,
            y: bar_y,
            color: HUD_TEXT_COLOR,
        },
    ];

    if game.state == GameState::Shop {
        texts.extend(build_shop(font_system, game));
    }

    HudDraw { quads, texts }
}

/// Shop screen: item list (price or owned-state per slot) plus a
/// highlight on `game.shop_cursor`'s entry. Reuses `super::make_line_buffer`
/// exactly like the HP/cash/score/level text above.
fn build_shop(font_system: &mut FontSystem, game: &Game) -> Vec<HudText> {
    let mut texts = Vec::with_capacity(SHOP_SLOT_COUNT + 1);

    let title = super::make_line_buffer(
        font_system,
        "SHOP",
        HUD_FONT_SIZE,
        HUD_LINE_HEIGHT,
        PLAYFIELD_WIDTH,
        Align::Center,
    );
    texts.push(HudText {
        buffer: title,
        x: 0.0,
        y: SHOP_TITLE_Y,
        color: HUD_TEXT_COLOR,
    });

    for slot in 0..SHOP_SLOT_COUNT {
        let highlighted = game.shop_cursor == slot;
        let prefix = if highlighted { "> " } else { "  " };
        let label = shop_item_label(&game.loadout, slot);
        let buffer = super::make_line_buffer(
            font_system,
            &format!("{prefix}{label}"),
            HUD_FONT_SIZE,
            HUD_LINE_HEIGHT,
            PLAYFIELD_WIDTH,
            Align::Center,
        );
        texts.push(HudText {
            buffer,
            x: 0.0,
            y: SHOP_ITEM_START_Y + slot as f32 * SHOP_ITEM_LINE_HEIGHT,
            color: if highlighted {
                SHOP_HIGHLIGHT_COLOR
            } else {
                SHOP_ITEM_COLOR
            },
        });
    }

    texts
}
