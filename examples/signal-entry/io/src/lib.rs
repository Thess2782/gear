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

// We can't depend on gstd because it declares panic handler, so we just use gcore.

#![no_std]

use gcore::errors::SignalCode;
use parity_scale_codec::{Decode, Encode};

#[derive(Debug, Encode, Decode)]
pub enum HandleAction {
    Simple,
    Wait,
    WaitAndPanic,
    WaitAndReserveWithPanic,
    WaitAndExit,
    WaitWithReserveAmountAndPanic(u64),
    Panic,
    Exit,
    Accumulate,
    OutOfGas,
    PanicInSignal,
    AcrossWaits,
    ZeroReserve,
    ForbiddenCallInSignal([u8; 32]),
    ForbiddenAction,
    SaveSignal(SignalCode),
    ExceedMemory,
    ExceedStackLimit,
    UnreachableInstruction,
    InvalidDebugCall,
    UnrecoverableExt,
    IncorrectFree,
    WaitWithoutSendingMessage,
    MemoryAccess,
}

pub const WAIT_AND_RESERVE_WITH_PANIC_GAS: u64 = 10_000_000_000;
