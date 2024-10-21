// This file is part of Gear.

// Copyright (C) 2024 Gear Technologies Inc.
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

use frame_support::{
    traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
    weights::Weight,
};
use sp_std::marker::PhantomData;
#[cfg(feature = "try-runtime")]
use {
    frame_support::ensure,
    sp_runtime::{
        codec::{Decode, Encode},
        TryRuntimeError,
    },
    sp_std::vec::Vec,
};

use crate::{CodeStorage, Config, OriginalCodeLenStorage, Pallet};
use gear_core::{code::InstrumentedCode, ids::CodeId};

const MIGRATE_FROM_VERSION: u16 = 10;
const MIGRATE_TO_VERSION: u16 = 11;
const ALLOWED_CURRENT_STORAGE_VERSION: u16 = 11;

pub struct CodeStructRefacroringMigration<T: Config>(PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for CodeStructRefacroringMigration<T> {
    fn on_runtime_upgrade() -> Weight {
        let onchain = Pallet::<T>::on_chain_storage_version();

        // 1 read for onchain storage version
        let mut weight = T::DbWeight::get().reads(1);
        let mut counter = 0;

        if onchain == MIGRATE_FROM_VERSION {
            let current = Pallet::<T>::current_storage_version();
            if current != ALLOWED_CURRENT_STORAGE_VERSION {
                log::error!("❌ Migration is not allowed for current storage version {current:?}.");
                return weight;
            }

            let update_to = StorageVersion::new(MIGRATE_TO_VERSION);
            log::info!("🚚 Running migration from {onchain:?} to {update_to:?}, current storage version is {current:?}.");

            CodeStorage::<T>::translate(|id: CodeId, code: v9::InstrumentedCode| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                counter += 1;

                OriginalCodeLenStorage::<T>::insert(id, code.original_code_len);

                let code = unsafe {
                    InstrumentedCode::new_unchecked(
                        code.code,
                        code.exports,
                        code.static_pages,
                        code.stack_end,
                        code.instantiated_section_sizes,
                        code.version,
                    )
                };

                Some(code)
            });

            // Put new storage version
            weight = weight.saturating_add(T::DbWeight::get().writes(1));

            update_to.put::<Pallet<T>>();

            log::info!("✅ Successfully migrated storage. {counter} codes have been migrated");
        } else {
            log::info!("🟠 Migration requires onchain version {MIGRATE_FROM_VERSION}, so was skipped for {onchain:?}");
        }

        weight
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
        let current = Pallet::<T>::current_storage_version();
        let onchain = Pallet::<T>::on_chain_storage_version();

        let res = if onchain == MIGRATE_FROM_VERSION {
            ensure!(
                current == ALLOWED_CURRENT_STORAGE_VERSION,
                "Current storage version is not allowed for migration, check migration code in order to allow it."
            );

            Some(v9::CodeStorage::<T>::iter().count() as u64)
        } else {
            None
        };

        Ok(res.encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
        if let Some(old_count) = Option::<u64>::decode(&mut state.as_ref())
            .map_err(|_| "`pre_upgrade` provided an invalid state")?
        {
            let count = CodeStorage::<T>::iter_keys().count() as u64;
            ensure!(old_count == count, "incorrect count of elements");
        }

        Ok(())
    }
}

mod v9 {
    use gear_core::{
        code::InstantiatedSectionSizes,
        message::DispatchKind,
        pages::{WasmPage, WasmPagesAmount},
    };
    use sp_runtime::{
        codec::{Decode, Encode},
        scale_info::TypeInfo,
    };
    use sp_std::{collections::btree_set::BTreeSet, vec::Vec};

    #[derive(Clone, Debug, Decode, Encode, TypeInfo)]
    pub struct InstrumentedCode {
        pub code: Vec<u8>,
        pub original_code_len: u32,
        pub exports: DispatchKind,
        pub static_pages: WasmPagesAmount,
        pub stack_end: Option<WasmPage>,
        pub instantiated_section_sizes: InstantiatedSectionSizes,
        pub version: u32,
    }

    #[cfg(feature = "try-runtime")]
    use {
        crate::{Config, Pallet},
        frame_support::{
            storage::types::StorageMap,
            traits::{PalletInfo, StorageInstance},
            Identity,
        },
        gear_core::ids::CodeId,
        sp_std::marker::PhantomData,
    };

    #[cfg(feature = "try-runtime")]
    pub struct CodeStoragePrefix<T>(PhantomData<T>);

    #[cfg(feature = "try-runtime")]
    impl<T: Config> StorageInstance for CodeStoragePrefix<T> {
        const STORAGE_PREFIX: &'static str = "CodeStorage";

        fn pallet_prefix() -> &'static str {
            <<T as frame_system::Config>::PalletInfo as PalletInfo>::name::<Pallet<T>>()
                .expect("No name found for the pallet in the runtime!")
        }
    }

    #[cfg(feature = "try-runtime")]
    pub type CodeStorage<T> = StorageMap<CodeStoragePrefix<T>, Identity, CodeId, InstrumentedCode>;
}

#[cfg(test)]
#[cfg(feature = "try-runtime")]
mod test {
    use super::*;
    use crate::mock::*;
    use frame_support::traits::StorageVersion;
    use gear_core::{code::InstantiatedSectionSizes, ids::CodeId, message::DispatchKind};
    use sp_runtime::traits::Zero;

    fn wat2wasm(s: &str) -> Vec<u8> {
        wabt::Wat2Wasm::new().convert(s).unwrap().as_ref().to_vec()
    }

    #[test]
    fn add_section_sizes_works() {
        new_test_ext().execute_with(|| {
            StorageVersion::new(MIGRATE_FROM_VERSION).put::<GearProgram>();

            let wat = r#"
                (module
                    (import "env" "memory" (memory 3))
                    (data (i32.const 0x20000) "gear")
                    (data (i32.const 0x20001) "gear")
                    (type (;36;) (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)))
                    (func $init)
                    (export "init" (func $init))
                    (func $sum (param i32 i32) (result i32)
                        local.get 0
                        local.get 1
                        i32.add
                    )
                    (global (mut i32) (i32.const 0))
                    (global (mut i32) (i32.const 0))
                    (global (mut i64) (i64.const 0))
                    (table 10 10 funcref)
                    (elem (i32.const 1) 0 0 0 0)
                )
            "#;

            let instantiated_section_sizes = unsafe { InstantiatedSectionSizes::zero() };

            let code = v9::InstrumentedCode {
                code: wat2wasm(wat),
                original_code_len: 100,
                exports: vec![DispatchKind::Init].into_iter().collect(),
                static_pages: 1.into(),
                stack_end: None,
                instantiated_section_sizes: instantiated_section_sizes.clone(),
                version: 1,
            };

            v9::CodeStorage::<Test>::insert(CodeId::from(1u64), code.clone());

            let state = CodeStructRefacroringMigration::<Test>::pre_upgrade().unwrap();
            let w = CodeStructRefacroringMigration::<Test>::on_runtime_upgrade();
            assert!(!w.is_zero());
            CodeStructRefacroringMigration::<Test>::post_upgrade(state).unwrap();

            let new_code = CodeStorage::<Test>::get(CodeId::from(1u64)).unwrap();
            assert_eq!(new_code.code(), code.code.as_slice());
            assert_eq!(new_code.exports(), &code.exports);
            assert_eq!(new_code.static_pages(), code.static_pages);
            assert_eq!(new_code.instruction_weights_version(), code.version);
            assert_eq!(new_code.stack_end(), None);
            assert_eq!(new_code.instantiated_section_sizes(), &instantiated_section_sizes);

            let original_code_len = OriginalCodeLenStorage::<Test>::get(CodeId::from(1u64)).unwrap();
            assert_eq!(original_code_len, code.original_code_len);
        });
    }
}
