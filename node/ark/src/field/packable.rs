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

//! Modified based on the [`plonky2`](https://github.com/0xPolygonZero/plonky2.git).

use crate::field::{packed::PackedField, types::Field};

/// Points us to the default packing for a particular field. There may me multiple choices of
/// PackedField for a particular Field (e.g. every Field is also a PackedField), but this is the
/// recommended one. The recommended packing varies by target_arch and target_feature.
pub trait Packable: Field {
    type Packing: PackedField<Scalar = Self>;
}

// TODO: uncomment the below upon adding the x86_64 specific code which
// has been omitted for now for the sake of controlling the PR size.
/*
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
impl Packable for crate::field::goldilocks_field::GoldilocksField {
    type Packing = crate::field::arch::x86_64::avx2_goldilocks_field::Avx2GoldilocksField;
}

#[cfg(not(all(target_arch = "x86_64", target_feature = "avx2")))]
*/
impl Packable for crate::field::goldilocks_field::GoldilocksField {
    type Packing = Self;
}
