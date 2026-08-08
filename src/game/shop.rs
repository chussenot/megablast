//! Shop economy: pure logic, no UI/window types -- `render::hud` reads
//! this state to draw the shop screen; buying/selling must be fully
//! unit-testable headless (spec).
//!
//! Owner: Wave 4 `shop` task. `game/mod.rs`'s Shop-state handling and
//! Wave 4's `shop-wiring` task (`render/hud.rs`) both depend on this
//! shape.

use super::weapons::Loadout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Cannon,
    SideShots,
    RearShot,
    Drone,
    Repair,
    ExtraLife,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopError {
    InsufficientFunds,
    MaxTier,
    AlreadyOwned,
    NotOwned,
    MaxDrones,
}

/// Cannon upgrade price depends on the tier being upgraded FROM (index
/// `tier - 1`): tier 1->2 costs `CANNON_PRICES[0]`, etc.
pub const CANNON_PRICES: [u32; 3] = [300, 600, 1000];
pub const SIDE_SHOTS_PRICE: u32 = 400;
pub const REAR_SHOT_PRICE: u32 = 250;
pub const DRONE_PRICE: u32 = 500;
pub const REPAIR_PRICE: u32 = 150;
pub const REPAIR_PERCENT: f32 = 0.25;
pub const EXTRA_LIFE_PRICE: u32 = 900;
pub const MAX_DRONES: u8 = 2;

#[derive(Debug, Clone, Default)]
pub struct Shop {
    pub cash: u32,
}

/// Buys `item`, mutating `loadout`/`lives`/`player_hp` and `shop.cash` on
/// success. TODO(wave4 `shop`): every path in the spec -- each item's
/// price/effect, repair capping at `max_hp`, and every rejection
/// (insufficient funds / max tier / already owned / max drones).
#[allow(clippy::too_many_arguments)]
pub fn buy(
    shop: &mut Shop,
    loadout: &mut Loadout,
    lives: &mut u32,
    player_hp: &mut f32,
    max_hp: f32,
    item: Item,
) -> Result<(), ShopError> {
    let _ = (shop, loadout, lives, player_hp, max_hp, item);
    Err(ShopError::InsufficientFunds)
}

/// Sells `item` back at half its price into `shop.cash` (spec).
/// TODO(wave4 `shop`).
pub fn sell(shop: &mut Shop, loadout: &mut Loadout, item: Item) -> Result<(), ShopError> {
    let _ = (shop, loadout, item);
    Err(ShopError::NotOwned)
}
