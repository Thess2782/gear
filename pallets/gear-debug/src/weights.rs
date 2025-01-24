// This file is part of Gear.

// Copyright (C) 2021-2025 Gear Technologies Inc.
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

//! Autogenerated weights for pallet_gear
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-08-25, STEPS: `[50, ]`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("local"), DB CACHE: 128

// Executed Command:
// ./target/release/gear
// benchmark
// --chain=local
// --execution=wasm
// --wasm-execution=compiled
// --pallet=pallet_gear_debug
// --extrinsic=*
// --steps
// 50
// --repeat
// 20
// --output
// .

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_gear.
pub trait WeightInfo {
    fn enable_debug_mode() -> Weight;
}

pub struct GearSupportWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for GearSupportWeight<T> {
    fn enable_debug_mode() -> Weight {
        Weight::zero().saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

impl WeightInfo for () {
    fn enable_debug_mode() -> Weight {
        Weight::zero().saturating_add(RocksDbWeight::get().writes(1_u64))
    }
}
