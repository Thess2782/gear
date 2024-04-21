// This file is part of Gear.

// Copyright (C) 2022-2024 Gear Technologies Inc.
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

//! Autogenerated weights for pallet_balances
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-04-09, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! CPU: `Intel(R) Xeon(R) Platinum 8375C CPU @ 2.90GHz`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("vara-dev"), DB CACHE: 1024

// Executed Command:
// ./target/production/gear benchmark pallet --chain=vara-dev --steps=50 --repeat=20 --pallet=pallet_balances --extrinsic=* --heap-pages=4096 --output=./scripts/benchmarking/weights-output/pallet_balances.rs --template=.maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_balances.
pub trait WeightInfo {
    fn transfer_allow_death() -> Weight;
    fn transfer_keep_alive() -> Weight;
    fn force_set_balance_creating() -> Weight;
    fn force_set_balance_killing() -> Weight;
    fn force_transfer() -> Weight;
    fn transfer_all() -> Weight;
    fn force_unreserve() -> Weight;
    fn upgrade_accounts(u: u32, ) -> Weight;
}

/// Weights for pallet_balances using the Gear node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_balances::WeightInfo for SubstrateWeight<T> {
    fn transfer_allow_death() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `103`
        //  Estimated: `6196`
        // Minimum execution time: 69_512_000 picoseconds.
        Weight::from_parts(70_074_000, 6196)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn transfer_keep_alive() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `3593`
        // Minimum execution time: 39_462_000 picoseconds.
        Weight::from_parts(39_985_000, 3593)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn force_set_balance_creating() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `103`
        //  Estimated: `3593`
        // Minimum execution time: 13_432_000 picoseconds.
        Weight::from_parts(13_760_000, 3593)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn force_set_balance_killing() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `103`
        //  Estimated: `3593`
        // Minimum execution time: 20_333_000 picoseconds.
        Weight::from_parts(20_599_000, 3593)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn force_transfer() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `206`
        //  Estimated: `8799`
        // Minimum execution time: 70_231_000 picoseconds.
        Weight::from_parts(71_685_000, 8799)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(3_u64))
    }
    fn transfer_all() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `3593`
        // Minimum execution time: 49_927_000 picoseconds.
        Weight::from_parts(50_809_000, 3593)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn force_unreserve() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `103`
        //  Estimated: `3593`
        // Minimum execution time: 16_358_000 picoseconds.
        Weight::from_parts(16_687_000, 3593)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    /// The range of component `u` is `[1, 1000]`.
    fn upgrade_accounts(u: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0 + u * (136 ±0)`
        //  Estimated: `990 + u * (2603 ±0)`
        // Minimum execution time: 16_077_000 picoseconds.
        Weight::from_parts(16_293_000, 990)
            // Standard Error: 10_333
            .saturating_add(Weight::from_parts(14_387_998, 0).saturating_mul(u.into()))
            .saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(u.into())))
            .saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(u.into())))
            .saturating_add(Weight::from_parts(0, 2603).saturating_mul(u.into()))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn transfer_allow_death() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `103`
        //  Estimated: `6196`
        // Minimum execution time: 69_512_000 picoseconds.
        Weight::from_parts(70_074_000, 6196)
            .saturating_add(RocksDbWeight::get().reads(2_u64))
            .saturating_add(RocksDbWeight::get().writes(2_u64))
    }
    fn transfer_keep_alive() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `3593`
        // Minimum execution time: 39_462_000 picoseconds.
        Weight::from_parts(39_985_000, 3593)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn force_set_balance_creating() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `103`
        //  Estimated: `3593`
        // Minimum execution time: 13_432_000 picoseconds.
        Weight::from_parts(13_760_000, 3593)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn force_set_balance_killing() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `103`
        //  Estimated: `3593`
        // Minimum execution time: 20_333_000 picoseconds.
        Weight::from_parts(20_599_000, 3593)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn force_transfer() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `206`
        //  Estimated: `8799`
        // Minimum execution time: 70_231_000 picoseconds.
        Weight::from_parts(71_685_000, 8799)
            .saturating_add(RocksDbWeight::get().reads(3_u64))
            .saturating_add(RocksDbWeight::get().writes(3_u64))
    }
    fn transfer_all() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `3593`
        // Minimum execution time: 49_927_000 picoseconds.
        Weight::from_parts(50_809_000, 3593)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    fn force_unreserve() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `103`
        //  Estimated: `3593`
        // Minimum execution time: 16_358_000 picoseconds.
        Weight::from_parts(16_687_000, 3593)
            .saturating_add(RocksDbWeight::get().reads(1_u64))
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    /// The range of component `u` is `[1, 1000]`.
    fn upgrade_accounts(u: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0 + u * (136 ±0)`
        //  Estimated: `990 + u * (2603 ±0)`
        // Minimum execution time: 16_077_000 picoseconds.
        Weight::from_parts(16_293_000, 990)
            // Standard Error: 10_333
            .saturating_add(Weight::from_parts(14_387_998, 0).saturating_mul(u.into()))
            .saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(u.into())))
            .saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(u.into())))
            .saturating_add(Weight::from_parts(0, 2603).saturating_mul(u.into()))
    }
}
