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

//! Autogenerated weights for pallet_gear_builtin
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-04-10, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! CPU: `Intel(R) Xeon(R) Platinum 8375C CPU @ 2.90GHz`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("vara-dev"), DB CACHE: 1024

// Executed Command:
// ./target/production/gear benchmark pallet --chain=vara-dev --steps=50 --repeat=20 --pallet=pallet_gear_builtin --extrinsic=* --heap-pages=4096 --output=./scripts/benchmarking/weights-output/pallet_gear_builtin.rs --template=.maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_gear_builtin.
pub trait WeightInfo {
    fn calculate_id() -> Weight;
    fn create_dispatcher() -> Weight;
    fn decode_bytes(a: u32, ) -> Weight;
    fn bls12_381_multi_miller_loop(c: u32, ) -> Weight;
    fn bls12_381_final_exponentiation() -> Weight;
    fn bls12_381_msm_g1(c: u32, ) -> Weight;
    fn bls12_381_msm_g2(c: u32, ) -> Weight;
    fn bls12_381_mul_projective_g1(c: u32, ) -> Weight;
    fn bls12_381_mul_projective_g2(c: u32, ) -> Weight;
}

/// Weights for pallet_gear_builtin using the Gear node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_gear_builtin::WeightInfo for SubstrateWeight<T> {
    fn calculate_id() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 628_000 picoseconds.
        Weight::from_parts(671_000, 0)
    }
    fn create_dispatcher() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 7_425_000 picoseconds.
        Weight::from_parts(7_530_000, 0)
    }
    /// The range of component `a` is `[1, 8388608]`.
    fn decode_bytes(a: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 410_000 picoseconds.
        Weight::from_parts(439_000, 0)
            // Standard Error: 0
            .saturating_add(Weight::from_parts(231, 0).saturating_mul(a.into()))
    }
    /// The range of component `c` is `[0, 100]`.
    fn bls12_381_multi_miller_loop(c: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 9_407_000 picoseconds.
        Weight::from_parts(566_088_359, 0)
            // Standard Error: 441_114
            .saturating_add(Weight::from_parts(220_794_711, 0).saturating_mul(c.into()))
    }
    fn bls12_381_final_exponentiation() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 771_031_000 picoseconds.
        Weight::from_parts(785_208_000, 0)
    }
    /// The range of component `c` is `[1, 1000]`.
    fn bls12_381_msm_g1(c: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 270_590_000 picoseconds.
        Weight::from_parts(801_084_085, 0)
            // Standard Error: 20_614
            .saturating_add(Weight::from_parts(5_946_232, 0).saturating_mul(c.into()))
    }
    /// The range of component `c` is `[1, 1000]`.
    fn bls12_381_msm_g2(c: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 701_049_000 picoseconds.
        Weight::from_parts(1_947_748_202, 0)
            // Standard Error: 54_805
            .saturating_add(Weight::from_parts(17_365_842, 0).saturating_mul(c.into()))
    }
    /// The range of component `c` is `[1, 100]`.
    fn bls12_381_mul_projective_g1(c: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 52_128_000 picoseconds.
        Weight::from_parts(52_559_000, 0)
            // Standard Error: 25_954
            .saturating_add(Weight::from_parts(57_109_399, 0).saturating_mul(c.into()))
    }
    /// The range of component `c` is `[1, 100]`.
    fn bls12_381_mul_projective_g2(c: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 160_561_000 picoseconds.
        Weight::from_parts(28_414_074, 0)
            // Standard Error: 32_279
            .saturating_add(Weight::from_parts(173_595_064, 0).saturating_mul(c.into()))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn calculate_id() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 628_000 picoseconds.
        Weight::from_parts(671_000, 0)
    }
    fn create_dispatcher() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 7_425_000 picoseconds.
        Weight::from_parts(7_530_000, 0)
    }
    /// The range of component `a` is `[1, 8388608]`.
    fn decode_bytes(a: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 410_000 picoseconds.
        Weight::from_parts(439_000, 0)
            // Standard Error: 0
            .saturating_add(Weight::from_parts(231, 0).saturating_mul(a.into()))
    }
    /// The range of component `c` is `[0, 100]`.
    fn bls12_381_multi_miller_loop(c: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 9_407_000 picoseconds.
        Weight::from_parts(566_088_359, 0)
            // Standard Error: 441_114
            .saturating_add(Weight::from_parts(220_794_711, 0).saturating_mul(c.into()))
    }
    fn bls12_381_final_exponentiation() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 771_031_000 picoseconds.
        Weight::from_parts(785_208_000, 0)
    }
    /// The range of component `c` is `[1, 1000]`.
    fn bls12_381_msm_g1(c: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 270_590_000 picoseconds.
        Weight::from_parts(801_084_085, 0)
            // Standard Error: 20_614
            .saturating_add(Weight::from_parts(5_946_232, 0).saturating_mul(c.into()))
    }
    /// The range of component `c` is `[1, 1000]`.
    fn bls12_381_msm_g2(c: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 701_049_000 picoseconds.
        Weight::from_parts(1_947_748_202, 0)
            // Standard Error: 54_805
            .saturating_add(Weight::from_parts(17_365_842, 0).saturating_mul(c.into()))
    }
    /// The range of component `c` is `[1, 100]`.
    fn bls12_381_mul_projective_g1(c: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 52_128_000 picoseconds.
        Weight::from_parts(52_559_000, 0)
            // Standard Error: 25_954
            .saturating_add(Weight::from_parts(57_109_399, 0).saturating_mul(c.into()))
    }
    /// The range of component `c` is `[1, 100]`.
    fn bls12_381_mul_projective_g2(c: u32, ) -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `0`
        //  Estimated: `0`
        // Minimum execution time: 160_561_000 picoseconds.
        Weight::from_parts(28_414_074, 0)
            // Standard Error: 32_279
            .saturating_add(Weight::from_parts(173_595_064, 0).saturating_mul(c.into()))
    }
}
