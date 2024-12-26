// This file is part of Gear.

// Copyright (C) 2021-2024 Gear Technologies Inc.
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

//! Some necessary self-contained primitives that need to be compiled as a part of the node.
//! Includes some Goldilocks finite field arithmetic used in Poseidon hashing logic,
//! as well as some Poseidon hashing funcitons over the Goldilocks field.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(target_arch = "x86_64", feature(stdarch_x86_avx512))]

extern crate alloc;

pub mod field;
pub mod hash;
pub mod util;
