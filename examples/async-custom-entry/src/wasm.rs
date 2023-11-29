// This file is part of Gear.

// Copyright (C) 2023 Gear Technologies Inc.
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

//! This program demonstrates the use of async `init` entry point, as well as custom entry point
//! functions for `handle_reply` and `handle_signal`, using of `gstd::async_init` and
//! `gstd::async_main` macros.
//!
//! `Init` is async and saves the source of the message it receives.
//!
//! `Handle` is async and goes into an infinite loop.
//!
//! `HandleReply` and `HandleSignal` are custom functions, defined as parameters in the
//! `gstd::async_init` macro. They send a message using [`send_bytes()`] containing their function name to the user saved
//! by the `Init` method.
//!
//! [`send_bytes()`]: msg::send_bytes

use gstd::{msg, ActorId};

static mut USER: ActorId = ActorId::zero();

#[gstd::async_init(handle_reply = my_handle_reply, handle_signal = my_handle_signal)]
async fn init() {
    gstd::Config::set_system_reserve(10_000_000_000).expect("Failed to set system reserve");

    unsafe { USER = msg::source() }
}

#[gstd::async_main]
async fn main() {
    #[allow(clippy::empty_loop)]
    loop {}
}

fn my_handle_reply() {
    unsafe {
        msg::send_bytes(USER, b"my_handle_reply", 0).unwrap();
    }
}

fn my_handle_signal() {
    unsafe {
        msg::send_bytes(USER, b"my_handle_signal", 0).unwrap();
    }
}
