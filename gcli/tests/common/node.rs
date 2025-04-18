// This file is part of Gear.
//
// Copyright (C) 2021-2025 Gear Technologies Inc.
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

//! Shared node.

use crate::common::{Args, Output, Result};

/// Convert self into `String`.
pub trait Convert<T> {
    fn convert(&self) -> T;
}

impl Convert<String> for Vec<u8> {
    fn convert(&self) -> String {
        String::from_utf8_lossy(self).to_string()
    }
}

/// Run node.
pub trait NodeExec {
    /// Exec command gcli with Node instance.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let node = Node::new();
    /// let args = Args::new("upload")
    ///              .flag("--code-only")
    ///              .program(env::wasm_bin("demo_fungible_token.opt.wasm"));
    /// let output = node.run(args)
    ///
    /// // ...
    /// ```
    fn run(&self, args: Args) -> Result<Output>;
}
