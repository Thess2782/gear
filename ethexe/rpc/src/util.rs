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

use anyhow::{Context, Result};
use hyper::header::HeaderValue;
use tower_http::cors::{AllowOrigin, CorsLayer};

pub(crate) fn try_into_cors(maybe_cors: Option<Vec<String>>) -> Result<CorsLayer> {
    if let Some(cors) = maybe_cors {
        let mut list = Vec::new();

        for origin in cors {
            let v = HeaderValue::from_str(&origin)
                .with_context(|| format!("invalid cors value - {origin}"))?;
            list.push(v);
        }

        Ok(CorsLayer::new().allow_origin(AllowOrigin::list(list)))
    } else {
        // allow all cors
        Ok(CorsLayer::permissive())
    }
}
