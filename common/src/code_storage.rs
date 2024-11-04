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

use super::*;
use crate::storage::MapStorage;
use gear_core::code::{CodeAndId, CodeMetadata, InstrumentedCode, InstrumentedCodeAndMetadata};

#[derive(Clone, Copy, Debug)]
pub enum Error {
    /// Code already exists in storage.
    DuplicateItem,
}

/// Trait to work with program binary codes in a storage.
pub trait CodeStorage {
    type InstrumentedCodeStorage: MapStorage<Key = CodeId, Value = InstrumentedCode>;
    type OriginalCodeStorage: MapStorage<Key = CodeId, Value = Vec<u8>>;
    type CodeMetadataStorage: MapStorage<Key = CodeId, Value = CodeMetadata>;

    /// Attempt to remove all items from all the associated maps.
    fn reset() {
        Self::CodeMetadataStorage::clear();
        Self::OriginalCodeStorage::clear();
        Self::InstrumentedCodeStorage::clear();
    }

    /// Add the code to the storage.
    fn add_code(code_and_id: CodeAndId) -> Result<(), Error> {
        let (code, code_id) = code_and_id.into_parts();
        let (original_code, instrumented_code, code_metadata) = code.into_parts();

        Self::InstrumentedCodeStorage::mutate(code_id, |maybe| {
            if maybe.is_some() {
                return Err(CodeStorageError::DuplicateItem);
            }

            Self::OriginalCodeStorage::insert(code_id, original_code);
            Self::CodeMetadataStorage::insert(code_id, code_metadata);

            *maybe = Some(instrumented_code);
            Ok(())
        })
    }

    /// Update the corresponding code and metadata in the storage.
    fn update_instrumented_code_and_metadata(
        code_id: CodeId,
        instrumented_code_and_metadata: InstrumentedCodeAndMetadata,
    ) {
        Self::InstrumentedCodeStorage::insert(
            code_id,
            instrumented_code_and_metadata.instrumented_code,
        );
        Self::CodeMetadataStorage::insert(code_id, instrumented_code_and_metadata.metadata);
    }

    /// Update the corresponding metadata in the storage.
    fn update_code_metadata(code_id: CodeId, metadata: CodeMetadata) {
        Self::CodeMetadataStorage::insert(code_id, metadata);
    }

    /// Returns true if the code associated with given id exists.
    fn exists(code_id: CodeId) -> bool {
        Self::InstrumentedCodeStorage::contains_key(&code_id)
    }

    /// Returns true if the code associated with given id was removed.
    ///
    /// If there is no code for the given id then false is returned.
    fn remove_code(code_id: CodeId) -> bool {
        Self::InstrumentedCodeStorage::mutate(code_id, |maybe| {
            if maybe.is_none() {
                return false;
            }

            Self::OriginalCodeStorage::remove(code_id);
            Self::CodeMetadataStorage::remove(code_id);

            *maybe = None;
            true
        })
    }

    fn get_instrumented_code(code_id: CodeId) -> Option<InstrumentedCode> {
        Self::InstrumentedCodeStorage::get(&code_id)
    }

    fn get_original_code(code_id: CodeId) -> Option<Vec<u8>> {
        Self::OriginalCodeStorage::get(&code_id)
    }

    fn get_code_metadata(code_id: CodeId) -> Option<CodeMetadata> {
        Self::CodeMetadataStorage::get(&code_id)
    }
}
