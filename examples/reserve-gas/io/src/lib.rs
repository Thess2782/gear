// This file is part of Gear.
//
// Copyright (C) 2024 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode};

pub const RESERVATION_AMOUNT: u64 = 50_000_000;
pub const REPLY_FROM_RESERVATION_PAYLOAD: &[u8; 5] = b"Hello";

#[derive(Debug, Encode, Decode)]
pub enum InitAction {
    Normal(Vec<(u64, u32)>),
    Wait,
    CheckArgs { mailbox_threshold: u64 },
    FreshReserveUnreserve,
}

#[derive(Debug, Encode, Decode)]
pub enum HandleAction {
    Unreserve,
    Exit,
    ReplyFromReservation,
    AddReservationToList(GasAmount, BlockCount),
    ConsumeReservationsFromList,
    RunInifitely,
    SendFromReservationAndUnreserve,
}

#[derive(Debug, Encode, Decode)]
pub enum ReplyAction {
    Panic,
    Exit,
}

pub type GasAmount = u64;
pub type BlockCount = u32;
