//! Per-area house SQM rates from `{world}/house-prices.ron`.
//! Areas: `houseareas.dat` / `houses.cc` `LoadHouseAreas`.
//! House → area: XML `name` matched to `houses.dat` `Name` / `Area`.
//! Size is the houses XML `size` attribute; rent = `SQMPrice * size`.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

/// One neighborhood rate (`THouseArea::SQMPrice`).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct HouseArea {
    pub id: u16,
    pub name: String,
    pub sqm: u32,
}

/// `house-prices.ron` next to the OTBM.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct HousePrices {
    pub areas: Vec<HouseArea>,
    /// House name (`forgotten-houses.xml` / `houses.dat` `Name`) → area id.
    #[serde(default)]
    pub houses: HashMap<String, u16>,
}

impl HousePrices {
    pub fn sqm_for_house(&self, name: &str) -> Option<u32> {
        let area_id = *self.houses.get(name)?;
        self.areas.iter().find(|a| a.id == area_id).map(|a| a.sqm)
    }
}

/// How to replace XML `rent=` using XML `size`.
#[derive(Debug, Clone)]
pub enum HouseRentPolicy {
    /// Leave `HouseXmlEntry::rent` / record rent as loaded.
    KeepXml,
    /// `rent = size * gold_per_sqm` (`housePriceEachSQM` ≥ 0).
    BlanketSqm(u32),
    /// `rent = area.sqm * size`.
    AreaPrices(HousePrices),
}

/// Parse `house-prices.ron`.
pub fn load_house_prices(path: &Path) -> Result<HousePrices> {
    let text = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    load_house_prices_str(&text, path)
}

pub fn load_house_prices_str(text: &str, path: &Path) -> Result<HousePrices> {
    let prices: HousePrices = ron::from_str(text).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    info!(
        file = %path.display(),
        areas = prices.areas.len(),
        houses = prices.houses.len(),
        "loaded house-prices.ron"
    );
    Ok(prices)
}

/// `Some(rent)` when the policy computes a value; `None` means keep existing rent.
pub fn compute_rent(house_name: &str, size: u32, policy: &HouseRentPolicy) -> Option<u32> {
    match policy {
        HouseRentPolicy::KeepXml => None,
        HouseRentPolicy::BlanketSqm(sqm) => Some(size.saturating_mul(*sqm)),
        HouseRentPolicy::AreaPrices(prices) => {
            Some(size.saturating_mul(prices.sqm_for_house(house_name)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parses_area_and_house_map() {
        let ron = r#"
HousePrices(
    areas: [
        HouseArea(id: 101, name: "Sunset Homes", sqm: 40),
        HouseArea(id: 120, name: "Fibula", sqm: 65),
    ],
    houses: {
        "Sunset Homes, Flat 01": 101,
    },
)
"#;
        let prices = load_house_prices_str(ron, Path::new("t.ron")).expect("parse");
        assert_eq!(prices.sqm_for_house("Sunset Homes, Flat 01"), Some(40));
        assert_eq!(prices.sqm_for_house("Spiritkeep"), None);
    }

    #[test]
    fn area_rent_uses_xml_size() {
        let prices = HousePrices {
            areas: vec![HouseArea {
                id: 101,
                name: "Sunset Homes".into(),
                sqm: 40,
            }],
            houses: HashMap::from([("Sunset Homes, Flat 01".into(), 101)]),
        };
        let policy = HouseRentPolicy::AreaPrices(prices);
        assert_eq!(
            compute_rent("Sunset Homes, Flat 01", 23, &policy),
            Some(920)
        );
        assert_eq!(compute_rent("Unknown", 23, &policy), None);
    }

    #[test]
    fn blanket_uses_xml_size() {
        assert_eq!(
            compute_rent("x", 23, &HouseRentPolicy::BlanketSqm(50)),
            Some(1150)
        );
        assert_eq!(compute_rent("x", 23, &HouseRentPolicy::KeepXml), None);
    }

    #[test]
    fn parses_world_house_prices_ron() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world/house-prices.ron");
        if !path.is_file() {
            return;
        }
        let prices = load_house_prices(&path).expect("house-prices.ron");
        assert!(prices.areas.len() >= 60);
        assert_eq!(prices.sqm_for_house("Sunset Homes, Flat 01"), Some(40));
        assert_eq!(prices.sqm_for_house("Spiritkeep"), Some(45));
        let fibula = prices.areas.iter().find(|a| a.id == 120).expect("Fibula");
        assert_eq!(fibula.sqm, 65);
    }
}
