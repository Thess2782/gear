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

#![no_std]

use core::array::IntoIter;
use gstd::ActorId;
use parity_scale_codec::{Decode, Encode};

#[derive(Debug, Decode, Encode)]
pub struct InputArgs {
    pub approver_first: ActorId,
    pub approver_second: ActorId,
    pub approver_third: ActorId,
}

impl InputArgs {
    pub fn from_two(first: impl Into<[u8; 32]>, second: impl Into<[u8; 32]>) -> Self {
        Self {
            approver_first: first.into().into(),
            approver_second: second.into().into(),
            approver_third: ActorId::zero(),
        }
    }

    pub fn iter(&self) -> IntoIter<&ActorId, 3> {
        [
            &self.approver_first,
            &self.approver_second,
            &self.approver_third,
        ]
        .into_iter()
    }
}

#[cfg(not(feature = "std"))]
mod wasm;
