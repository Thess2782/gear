// This file is part of Gear.

// Copyright (C) 2021-2025 Gear Technologies Inc.
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

use crate as pallet_gear_debug;
use common::storage::Limiter;
use frame_support::{
    construct_runtime,
    dispatch::DispatchClass,
    parameter_types,
    traits::{ConstU32, FindAuthor, Get, OnFinalize, OnInitialize},
    weights::Weight,
};
use frame_support_test::TestRandomness;
use frame_system::{self as system, limits::BlockWeights, pallet_prelude::BlockNumberFor};
use pallet_gear::GasAllowanceOf;
use primitive_types::H256;
use sp_core::ConstBool;
use sp_runtime::{
    traits::{BlakeTwo256, ConstU64, IdentityLookup},
    BuildStorage,
};
use sp_std::convert::{TryFrom, TryInto};

type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;
pub type BlockNumber = BlockNumberFor<Test>;
type Balance = u128;

pub const BLOCK_AUTHOR: AccountId = 255;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const ExistentialDeposit: Balance = 1;
}

impl pallet_gear_debug::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type CodeStorage = GearProgram;
    type ProgramStorage = GearProgram;
    type Messenger = GearMessenger;
}

parameter_types! {
    pub const OutgoingLimit: u32 = 1024;
    pub const OutgoingBytesLimit: u32 = 64 * 1024 * 1024;
    pub const BlockGasLimit: u64 = 100_000_000_000;
    pub const PerformanceMultiplier: u32 = 100;
}

parameter_types! {
    pub RentFreePeriod: BlockNumber = 1_000;
    pub RentCostPerBlock: Balance = 11;
    pub ResumeMinimalPeriod: BlockNumber = 100;
    pub ResumeSessionDuration: BlockNumber = 1_000;
    pub const BankAddress: AccountId = 15082001;
    pub const GasMultiplier: common::GasMultiplier<Balance, u64> = common::GasMultiplier::ValuePerGas(100);
    pub ReserveThreshold: BlockNumber = 1;
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Test
    {
        System: system,
        GearDebug: pallet_gear_debug,
        Balances: pallet_balances,
        Authorship: pallet_authorship,
        Timestamp: pallet_timestamp,
        GearProgram: pallet_gear_program,
        GearMessenger: pallet_gear_messenger,
        GearScheduler: pallet_gear_scheduler,
        GearBank: pallet_gear_bank,
        Gear: pallet_gear,
        GearGas: pallet_gear_gas,
    }
);

common::impl_pallet_system!(Test, DbWeight = (), BlockWeights = ());
common::impl_pallet_balances!(Test);
common::impl_pallet_authorship!(Test);
common::impl_pallet_timestamp!(Test);
pallet_gear_program::impl_config!(Test);
pallet_gear_messenger::impl_config!(Test, CurrentBlockNumber = Gear);
pallet_gear_scheduler::impl_config!(Test);
pallet_gear_bank::impl_config!(Test);
pallet_gear::impl_config!(Test, DebugInfo = pallet_gear_debug::Pallet<Test>, ProgramRentEnabled = ConstBool<false>);
pallet_gear_gas::impl_config!(Test);

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 100_000_000_000_000_u128),
            (2, 2_000_u128),
            (BLOCK_AUTHOR, 1_000_u128),
            (BankAddress::get(), ExistentialDeposit::get()),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
        Gear::on_initialize(System::block_number());
    });
    ext
}

pub fn run_to_block(n: u64, remaining_weight: Option<u64>) {
    while System::block_number() < n {
        System::on_finalize(System::block_number());
        GearBank::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        GearGas::on_initialize(System::block_number());
        GearMessenger::on_initialize(System::block_number());
        Gear::on_initialize(System::block_number());
        GearBank::on_initialize(System::block_number());

        if let Some(remaining_weight) = remaining_weight {
            GasAllowanceOf::<Test>::put(remaining_weight);
            let max_block_weight =
                <<Test as frame_system::Config>::BlockWeights as Get<BlockWeights>>::get()
                    .max_block;
            System::register_extra_weight_unchecked(
                max_block_weight.saturating_sub(Weight::from_parts(remaining_weight, 0)),
                frame_support::dispatch::DispatchClass::Normal,
            );
        }

        // Spend the maximum weight of the block to account for the weight of Gear::run() in the current block.
        let max_block_weight =
            <<Test as frame_system::Config>::BlockWeights as Get<BlockWeights>>::get().max_block;
        System::register_extra_weight_unchecked(max_block_weight, DispatchClass::Mandatory);
        Gear::run(frame_support::dispatch::RawOrigin::None.into(), None).unwrap();
        Gear::on_finalize(System::block_number());
        GearBank::on_finalize(System::block_number());

        assert!(!System::events().iter().any(|e| {
            matches!(
                e.event,
                RuntimeEvent::Gear(pallet_gear::Event::QueueNotProcessed)
            )
        }))
    }
}

pub fn run_to_next_block(remaining_weight: Option<u64>) {
    run_to_block(System::block_number() + 1, remaining_weight);
}
