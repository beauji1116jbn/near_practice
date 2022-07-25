use near_units::parse_near;

pub const FT_WASM: &[u8] = include_bytes!("../../res/ft.wasm");
pub const AMM_WASM: &[u8] = include_bytes!("../../res/amm.wasm");
pub const GAS_MAX: u64 = 300000000000000;
pub const FT_INIT_SUPPLY: u128 = parse_near!("1,000,000,000 N");
