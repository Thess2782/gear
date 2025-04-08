// This file is part of Gear.

// Copyright (C) 2022-2025 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Autogenerated weights for pallet_gear_eth_bridge
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 43.0.0
//! DATE: 2024-11-25, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! CPU: `Intel(R) Xeon(R) Platinum 8375C CPU @ 2.90GHz`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("vara-dev"), DB CACHE: 1024

// Executed Command:
// ./target/production/gear benchmark pallet --chain=vara-dev --steps=50 --repeat=20 --pallet=pallet_gear_eth_bridge --extrinsic=* --heap-pages=4096 --output=./scripts/benchmarking/weights-output/pallet_gear_eth_bridge.rs --template=.maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_gear_eth_bridge.
pub trait WeightInfo {
    fn pause() -> Weight;
    fn unpause() -> Weight;
    fn set_fee() -> Weight;
    fn send_eth_message() -> Weight;
}

/// Weights for pallet_gear_eth_bridge using the Gear node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_gear_eth_bridge::WeightInfo for SubstrateWeight<T> {
    fn pause() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `121`
        //  Estimated: `1486`
        // Minimum execution time: 11_194_000 picoseconds.
        Weight::from_parts(11_649_000, 1486)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn unpause() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `100`
        //  Estimated: `1486`
        // Minimum execution time: 10_317_000 picoseconds.
        Weight::from_parts(10_843_000, 1486)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_fee() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 2_003_000 picoseconds.
        Weight::from_parts(2_164_000, 0)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn send_eth_message() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `121`
        //  Estimated: `67023`
        // Minimum execution time: 92_875_000 picoseconds.
        Weight::from_parts(93_589_000, 67023)
            .saturating_add(T::DbWeight::get().reads(4_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn pause() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `121`
        //  Estimated: `1486`
        // Minimum execution time: 11_194_000 picoseconds.
        Weight::from_parts(11_649_000, 1486)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn unpause() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `100`
        //  Estimated: `1486`
        // Minimum execution time: 10_317_000 picoseconds.
        Weight::from_parts(10_843_000, 1486)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn set_fee() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 2_003_000 picoseconds.
        Weight::from_parts(2_164_000, 0)
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn send_eth_message() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `121`
        //  Estimated: `67023`
        // Minimum execution time: 92_875_000 picoseconds.
        Weight::from_parts(93_589_000, 67023)
            .saturating_add(RocksDbWeight::get().reads(4_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }
}
