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

/// Price to upgrade the cannon from its current tier, or `None` if
/// already at the max tier (4).
fn cannon_price(tier: u8) -> Option<u32> {
    CANNON_PRICES.get(tier as usize - 1).copied()
}

/// Buys `item`, mutating `loadout`/`lives`/`player_hp` and `shop.cash` on
/// success.
#[allow(clippy::too_many_arguments)]
pub fn buy(
    shop: &mut Shop,
    loadout: &mut Loadout,
    lives: &mut u32,
    player_hp: &mut f32,
    max_hp: f32,
    item: Item,
) -> Result<(), ShopError> {
    let price = match item {
        Item::Cannon => cannon_price(loadout.cannon_tier).ok_or(ShopError::MaxTier)?,
        Item::SideShots => {
            if loadout.has_side {
                return Err(ShopError::AlreadyOwned);
            }
            SIDE_SHOTS_PRICE
        }
        Item::RearShot => {
            if loadout.has_rear {
                return Err(ShopError::AlreadyOwned);
            }
            REAR_SHOT_PRICE
        }
        Item::Drone => {
            if loadout.drones >= MAX_DRONES {
                return Err(ShopError::MaxDrones);
            }
            DRONE_PRICE
        }
        Item::Repair => REPAIR_PRICE,
        Item::ExtraLife => EXTRA_LIFE_PRICE,
    };
    if shop.cash < price {
        return Err(ShopError::InsufficientFunds);
    }
    shop.cash -= price;
    match item {
        Item::Cannon => loadout.cannon_tier += 1,
        Item::SideShots => loadout.has_side = true,
        Item::RearShot => loadout.has_rear = true,
        Item::Drone => loadout.drones += 1,
        Item::Repair => *player_hp = (*player_hp + max_hp * REPAIR_PERCENT).min(max_hp),
        Item::ExtraLife => *lives += 1,
    }
    Ok(())
}

/// Sells `item` back at half its price into `shop.cash` (spec). `Repair`
/// and `ExtraLife` are consumed instantly rather than owned, so they can
/// never be sold back -- always `NotOwned`.
pub fn sell(shop: &mut Shop, loadout: &mut Loadout, item: Item) -> Result<(), ShopError> {
    let price = match item {
        Item::Cannon => {
            if loadout.cannon_tier <= 1 {
                return Err(ShopError::NotOwned);
            }
            cannon_price(loadout.cannon_tier - 1).expect("tier - 1 is always a valid index")
        }
        Item::SideShots => {
            if !loadout.has_side {
                return Err(ShopError::NotOwned);
            }
            SIDE_SHOTS_PRICE
        }
        Item::RearShot => {
            if !loadout.has_rear {
                return Err(ShopError::NotOwned);
            }
            REAR_SHOT_PRICE
        }
        Item::Drone => {
            if loadout.drones == 0 {
                return Err(ShopError::NotOwned);
            }
            DRONE_PRICE
        }
        Item::Repair | Item::ExtraLife => return Err(ShopError::NotOwned),
    };
    shop.cash += price / 2;
    match item {
        Item::Cannon => loadout.cannon_tier -= 1,
        Item::SideShots => loadout.has_side = false,
        Item::RearShot => loadout.has_rear = false,
        Item::Drone => loadout.drones -= 1,
        Item::Repair | Item::ExtraLife => unreachable!(),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_loadout(cash: u32) -> (Shop, Loadout, u32, f32, f32) {
        (Shop { cash }, Loadout::default(), 3, 100.0, 100.0)
    }

    #[test]
    fn buy_cannon_success_and_price_progression() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(300);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::Cannon
            ),
            Ok(())
        );
        assert_eq!(loadout.cannon_tier, 2);
        assert_eq!(shop.cash, 0);
    }

    #[test]
    fn buy_cannon_insufficient_funds() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(299);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::Cannon
            ),
            Err(ShopError::InsufficientFunds)
        );
        assert_eq!(loadout.cannon_tier, 1);
        assert_eq!(shop.cash, 299);
    }

    #[test]
    fn buy_cannon_max_tier() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(u32::MAX);
        loadout.cannon_tier = 4;
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::Cannon
            ),
            Err(ShopError::MaxTier)
        );
        assert_eq!(loadout.cannon_tier, 4);
    }

    #[test]
    fn buy_side_shots_success_and_already_owned() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(SIDE_SHOTS_PRICE);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::SideShots
            ),
            Ok(())
        );
        assert!(loadout.has_side);
        assert_eq!(shop.cash, 0);
        shop.cash = SIDE_SHOTS_PRICE;
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::SideShots
            ),
            Err(ShopError::AlreadyOwned)
        );
    }

    #[test]
    fn buy_side_shots_insufficient_funds() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(SIDE_SHOTS_PRICE - 1);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::SideShots
            ),
            Err(ShopError::InsufficientFunds)
        );
        assert!(!loadout.has_side);
    }

    #[test]
    fn buy_rear_shot_success_and_already_owned() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(REAR_SHOT_PRICE);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::RearShot
            ),
            Ok(())
        );
        assert!(loadout.has_rear);
        shop.cash = REAR_SHOT_PRICE;
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::RearShot
            ),
            Err(ShopError::AlreadyOwned)
        );
    }

    #[test]
    fn buy_rear_shot_insufficient_funds() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(REAR_SHOT_PRICE - 1);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::RearShot
            ),
            Err(ShopError::InsufficientFunds)
        );
        assert!(!loadout.has_rear);
    }

    #[test]
    fn buy_drone_success_and_max_drones() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(DRONE_PRICE * 2);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::Drone
            ),
            Ok(())
        );
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::Drone
            ),
            Ok(())
        );
        assert_eq!(loadout.drones, MAX_DRONES);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::Drone
            ),
            Err(ShopError::MaxDrones)
        );
    }

    #[test]
    fn buy_drone_insufficient_funds() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(DRONE_PRICE - 1);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::Drone
            ),
            Err(ShopError::InsufficientFunds)
        );
        assert_eq!(loadout.drones, 0);
    }

    #[test]
    fn buy_repair_heals_and_caps_at_max_hp() {
        let (mut shop, mut loadout, mut lives, _, max_hp) = full_loadout(REPAIR_PRICE);
        let mut hp = 90.0;
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::Repair
            ),
            Ok(())
        );
        // 90 + 25% of 100 = 115, capped at 100.
        assert_eq!(hp, 100.0);
        assert_eq!(shop.cash, 0);
    }

    #[test]
    fn buy_repair_insufficient_funds() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(REPAIR_PRICE - 1);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::Repair
            ),
            Err(ShopError::InsufficientFunds)
        );
        assert_eq!(hp, 100.0);
    }

    #[test]
    fn buy_extra_life_success() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(EXTRA_LIFE_PRICE);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::ExtraLife
            ),
            Ok(())
        );
        assert_eq!(lives, 4);
        assert_eq!(shop.cash, 0);
    }

    #[test]
    fn buy_extra_life_insufficient_funds() {
        let (mut shop, mut loadout, mut lives, mut hp, max_hp) = full_loadout(EXTRA_LIFE_PRICE - 1);
        assert_eq!(
            buy(
                &mut shop,
                &mut loadout,
                &mut lives,
                &mut hp,
                max_hp,
                Item::ExtraLife
            ),
            Err(ShopError::InsufficientFunds)
        );
        assert_eq!(lives, 3);
    }

    #[test]
    fn sell_cannon_success_and_not_owned_at_tier_one() {
        let mut shop = Shop { cash: 0 };
        let mut loadout = Loadout {
            cannon_tier: 2,
            ..Loadout::default()
        };
        assert_eq!(sell(&mut shop, &mut loadout, Item::Cannon), Ok(()));
        assert_eq!(loadout.cannon_tier, 1);
        assert_eq!(shop.cash, CANNON_PRICES[0] / 2);
        assert_eq!(
            sell(&mut shop, &mut loadout, Item::Cannon),
            Err(ShopError::NotOwned)
        );
    }

    #[test]
    fn sell_side_shots_success_and_not_owned() {
        let mut shop = Shop { cash: 0 };
        let mut loadout = Loadout {
            has_side: true,
            ..Loadout::default()
        };
        assert_eq!(sell(&mut shop, &mut loadout, Item::SideShots), Ok(()));
        assert!(!loadout.has_side);
        assert_eq!(shop.cash, SIDE_SHOTS_PRICE / 2);
        assert_eq!(
            sell(&mut shop, &mut loadout, Item::SideShots),
            Err(ShopError::NotOwned)
        );
    }

    #[test]
    fn sell_rear_shot_success_and_not_owned() {
        let mut shop = Shop { cash: 0 };
        let mut loadout = Loadout {
            has_rear: true,
            ..Loadout::default()
        };
        assert_eq!(sell(&mut shop, &mut loadout, Item::RearShot), Ok(()));
        assert!(!loadout.has_rear);
        assert_eq!(shop.cash, REAR_SHOT_PRICE / 2);
        assert_eq!(
            sell(&mut shop, &mut loadout, Item::RearShot),
            Err(ShopError::NotOwned)
        );
    }

    #[test]
    fn sell_drone_success_and_not_owned() {
        let mut shop = Shop { cash: 0 };
        let mut loadout = Loadout {
            drones: 1,
            ..Loadout::default()
        };
        assert_eq!(sell(&mut shop, &mut loadout, Item::Drone), Ok(()));
        assert_eq!(loadout.drones, 0);
        assert_eq!(shop.cash, DRONE_PRICE / 2);
        assert_eq!(
            sell(&mut shop, &mut loadout, Item::Drone),
            Err(ShopError::NotOwned)
        );
    }

    #[test]
    fn sell_repair_and_extra_life_are_never_owned() {
        let mut shop = Shop { cash: 0 };
        let mut loadout = Loadout::default();
        assert_eq!(
            sell(&mut shop, &mut loadout, Item::Repair),
            Err(ShopError::NotOwned)
        );
        assert_eq!(
            sell(&mut shop, &mut loadout, Item::ExtraLife),
            Err(ShopError::NotOwned)
        );
    }
}
