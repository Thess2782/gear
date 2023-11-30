// This file is part of Gear.

// Copyright (C) 2021-2023 Gear Technologies Inc.
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

//! This program contains a state, which starts at 0, every time it gets incremented, and then the
//! program waits, when it reaches 2, it wakes the two waiting messages that started on states 0
//! and 1, and the messages now have state 3, where they send a message to the source containing
//! their message id as the payload.

// for panic/oom handlers
extern crate gstd;

use gcore::{exec, msg, MessageId};

static mut STATE: u32 = 0;
static mut MSG_ID_1: MessageId = MessageId::zero();
static mut MSG_ID_2: MessageId = MessageId::zero();

#[no_mangle]
extern "C" fn handle() {
    let state = unsafe { &mut STATE };
    gstd::debug!("{state}");

    match *state {
        0 => {
            *state = 1;
            unsafe { MSG_ID_1 = msg::id() };
            exec::wait();
        }
        1 => {
            *state = 2;
            unsafe { MSG_ID_2 = msg::id() };
            exec::wait();
        }
        2 => {
            *state = 3;
            exec::wake(unsafe { MSG_ID_1 }).unwrap();
            exec::wake(unsafe { MSG_ID_2 }).unwrap();
        }
        _ => {
            msg::send(msg::source(), msg::id().as_slice(), 0).unwrap();
        }
    }
}
