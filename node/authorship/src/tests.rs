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

// Modified implementation of the basic block-authorship logic from
// https://github.com/paritytech/substrate/tree/master/client/basic-authorship.
// The block proposer explicitly pushes the `pallet_gear::run`
// extrinsic at the end of each block.

#![allow(clippy::redundant_clone)]
#![allow(unused_mut)]

use crate::{authorship::MAX_SKIPPED_TRANSACTIONS, block_builder::BlockBuilder, ProposerFactory};

use codec::{Decode, Encode};
use common::Program;
use core::convert::TryFrom;
use demo_constructor::{Calls, Scheme, WASM_BINARY};
use frame_support::{assert_ok, storage::storage_prefix, traits::PalletInfoAccess};
use futures::executor::block_on;
use gear_runtime_common::constants::BANK_ADDRESS;
use pallet_gear_rpc_runtime_api::GearApi;
use runtime_primitives::BlockNumber;
use sc_client_api::{Backend as _, ExecutionStrategy};
use sc_service::client::Client;
use sc_transaction_pool::BasicPool;
use sc_transaction_pool_api::{
    ChainEvent, MaintainedTransactionPool, TransactionPool, TransactionSource,
};
use sp_api::{ApiExt, Core, ProvideRuntimeApi, StateBackend};
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Environment, Proposer};
use sp_consensus_babe::{
    digests::{PreDigest, SecondaryPlainPreDigest},
    Slot, BABE_ENGINE_ID,
};
use sp_inherents::InherentDataProvider;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Header as HeaderT, NumberFor},
    Digest, DigestItem, OpaqueExtrinsic, Perbill, Percent,
};
use sp_timestamp::Timestamp;
use std::{
    ops::Deref,
    sync::Arc,
    time::{self, SystemTime, UNIX_EPOCH},
};
use testing::{
    client::{ClientBlockImportExt, TestClientBuilder, TestClientBuilderExt},
    keyring::{alice, bob, sign, signed_extra, CheckedExtrinsic},
};
use vara_runtime::{
    AccountId, Runtime, RuntimeApi as RA, RuntimeCall, UncheckedExtrinsic, SLOT_DURATION, VERSION,
};

const SOURCE: TransactionSource = TransactionSource::External;
const DEFAULT_GAS_LIMIT: u64 = 10_000_000_000;

fn chain_event<B: BlockT>(header: B::Header) -> ChainEvent<B>
where
    NumberFor<B>: From<u32>,
{
    ChainEvent::NewBestBlock {
        hash: header.hash(),
        tree_route: None,
    }
}

fn pre_digest(slot: u64, authority_index: u32) -> Digest {
    Digest {
        logs: vec![DigestItem::PreRuntime(
            BABE_ENGINE_ID,
            PreDigest::SecondaryPlain(SecondaryPlainPreDigest {
                slot: Slot::from(slot),
                authority_index,
            })
            .encode(),
        )],
    }
}

fn checked_extrinsics<F>(
    n: u32,
    signer: AccountId,
    starting_nonce: u32,
    f: F,
) -> Vec<CheckedExtrinsic>
where
    F: Fn() -> RuntimeCall,
{
    let last_nonce = starting_nonce + n;
    (starting_nonce..last_nonce)
        .map(|nonce| CheckedExtrinsic {
            signed: Some((signer.clone(), signed_extra(nonce))),
            function: f(),
        })
        .collect()
}

fn sign_extrinsics<E>(
    extrinsics: Vec<CheckedExtrinsic>,
    spec_version: u32,
    tx_version: u32,
    genesis_hash: [u8; 32],
) -> Vec<E>
where
    E: From<UncheckedExtrinsic>,
{
    extrinsics
        .into_iter()
        .map(|x| sign(x, spec_version, tx_version, genesis_hash).into())
        .collect()
}

fn salt() -> [u8; 16] {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_le_bytes()
}

enum TestCall {
    DepositToBank,
    Noop,
    InitLoop(u64),
    ToggleRunQueue(bool),
    ExhaustResources,
}

struct CallBuilder {
    call: TestCall,
}
impl CallBuilder {
    pub fn deposit_to_bank() -> Self {
        Self {
            call: TestCall::DepositToBank,
        }
    }
    pub fn noop() -> Self {
        Self {
            call: TestCall::Noop,
        }
    }
    pub fn long_init(count: u64) -> Self {
        Self {
            call: TestCall::InitLoop(count),
        }
    }
    pub fn toggle_run_queue(value: bool) -> Self {
        Self {
            call: TestCall::ToggleRunQueue(value),
        }
    }
    pub fn exhaust_resources() -> Self {
        Self {
            call: TestCall::ExhaustResources,
        }
    }
    fn build(self) -> RuntimeCall {
        match self.call {
            TestCall::DepositToBank => RuntimeCall::Sudo(pallet_sudo::Call::sudo {
                call: Box::new(RuntimeCall::Balances(pallet_balances::Call::set_balance {
                    who: sp_runtime::MultiAddress::Id(AccountId::from(BANK_ADDRESS)),
                    new_free: 1_000_000_000_000_000,
                    new_reserved: 0,
                })),
            }),
            TestCall::Noop => RuntimeCall::Gear(pallet_gear::Call::upload_program {
                code: WASM_BINARY.to_vec(),
                salt: salt().to_vec(),
                init_payload: Scheme::direct(Calls::builder().noop()).encode(),
                gas_limit: DEFAULT_GAS_LIMIT,
                value: 0,
                keep_alive: false,
            }),
            TestCall::InitLoop(count) => RuntimeCall::Gear(pallet_gear::Call::upload_program {
                code: WASM_BINARY.to_vec(),
                salt: salt().to_vec(),
                init_payload: Scheme::direct(Calls::builder().write_in_loop(count)).encode(),
                gas_limit: DEFAULT_GAS_LIMIT,
                value: 0,
                keep_alive: false,
            }),
            TestCall::ToggleRunQueue(value) => RuntimeCall::Sudo(pallet_sudo::Call::sudo {
                call: Box::new(RuntimeCall::Gear(pallet_gear::Call::set_execute_inherent {
                    value,
                })),
            }),
            TestCall::ExhaustResources => {
                // Using 75% of the max possible weight so that two such calls will inevitably
                // exhaust block resources while one call will very likely fit in.
                RuntimeCall::GearDebug(pallet_gear_debug::Call::exhaust_block_resources {
                    fraction: Percent::from_percent(75),
                })
            }
        }
    }
}

pub(crate) fn init_logger() {
    let _ = env_logger::Builder::from_default_env()
        .format_module_path(false)
        .format_level(true)
        .try_init();
}

macro_rules! init {
    {
        $client:ident,
        $backend:ident,
        $txpool:ident,
        $spawner:ident,
        $genesis_hash:ident
    } => {
        let client_builder =
            TestClientBuilder::new().set_execution_strategy(ExecutionStrategy::NativeWhenPossible);
        let $backend = client_builder.backend();
        let mut $client = Arc::new(client_builder.build());
        let $spawner = sp_core::testing::TaskExecutor::new();
        let $txpool = BasicPool::new_full(
            Default::default(),
            true.into(),
            None,
            $spawner.clone(),
            $client.clone(),
        );

        let $genesis_hash =
            <[u8; 32]>::try_from(&$client.info().best_hash[..]).expect("H256 is a 32 byte type");
    }
}

macro_rules! submit_txs {
    {
        $client:ident,
        $txpool:ident,
        $block_id:expr,
        $extrinsics:expr
    } => {
        block_on($txpool.submit_at(&$block_id, SOURCE, $extrinsics)).unwrap();

        block_on(
            $txpool.maintain(chain_event(
                $client
                    .header(
                        $client
                            .block_hash_from_id(&$block_id)
                            .unwrap()
                            .unwrap(),
                    )
                    .expect("header get error")
                    .expect("there should be header"),
            )),
        );
    }
}

macro_rules! propose_block {
    {
        $client:ident,
        $backend:ident,
        $txpool:ident,
        $spawner:ident,
        $best_hash:ident,
        $block_id:expr,
        $now:expr,
        $timestamp:expr,
        $max_duration:expr,
        $max_gas:expr,
        $proposal:ident
    } => {
        let mut proposer_factory = ProposerFactory::new(
            $spawner.clone(),
            $client.clone(),
            $backend.clone(),
            $txpool.clone(),
            None,
            None,
            $max_gas,
        );

        let timestamp_provider = sp_timestamp::InherentDataProvider::new($timestamp);
        let time_slot = $timestamp.as_millis() / SLOT_DURATION;
        let inherent_data =
            block_on(timestamp_provider.create_inherent_data()).expect("Create inherent data failed");

        let proposer = proposer_factory.init_with_now(
            &$client.expect_header(
                $client
                    .block_hash_from_id(&$block_id)
                    .unwrap()
                    .unwrap()
                ).expect("There must be a header"),
            $now,
        );

        let $proposal = block_on(proposer.propose(
            inherent_data,
            pre_digest(time_slot, 0),
            time::Duration::from_millis($max_duration),
            None,
        ))
        .unwrap();

        // Importing last block
        block_on($client.import(BlockOrigin::Own, $proposal.block.clone())).unwrap();

        let $best_hash = $client.info().best_hash;
        assert_eq!($best_hash, $proposal.block.hash());
    }
}

#[test]
fn custom_extrinsic_is_placed_in_each_block() {
    init_logger();

    init!(client, backend, txpool, spawner, genesis_hash);

    let extrinsics = sign_extrinsics(
        checked_extrinsics(1, bob(), 0_u32, || CallBuilder::noop().build()),
        VERSION.spec_version,
        VERSION.transaction_version,
        genesis_hash,
    );

    submit_txs!(client, txpool, BlockId::number(0), extrinsics);
    assert_eq!(txpool.ready().count(), 1);

    propose_block!(
        client,
        backend,
        txpool,
        spawner,
        best_hash,
        BlockId::number(0),
        Box::new(time::Instant::now),
        Timestamp::current(),
        1500_u64,
        None,
        proposal
    );
    // then
    // block should have exactly 3 txs: an inherent (timestamp), a normal and a mandatory one
    assert_eq!(proposal.block.extrinsics().len(), 3);
}

#[test]
fn proposed_storage_changes_match_execute_block_storage_changes() {
    init_logger();

    init!(client, backend, txpool, spawner, genesis_hash);

    let extrinsics = sign_extrinsics(
        checked_extrinsics(1, bob(), 0_u32, || CallBuilder::noop().build()),
        VERSION.spec_version,
        VERSION.transaction_version,
        genesis_hash,
    );
    submit_txs!(client, txpool, BlockId::number(0), extrinsics);

    let timestamp = Timestamp::current();

    propose_block!(
        client,
        backend,
        txpool,
        spawner,
        best_hash,
        BlockId::number(0),
        Box::new(time::Instant::now),
        timestamp,
        1500_u64,
        None,
        proposal
    );
    // then
    // 1 inherent + 1 signed extrinsic + 1 terminal unsigned one
    assert_eq!(proposal.block.extrinsics().len(), 3);

    let api = client.runtime_api();
    api.execute_block(genesis_hash.into(), proposal.block)
        .unwrap();
    let state = backend.state_at(best_hash).unwrap();

    let storage_changes = api.into_storage_changes(&state, best_hash).unwrap();

    assert_eq!(
        proposal.storage_changes.transaction_storage_root,
        storage_changes.transaction_storage_root,
    );

    let queue_head_key = storage_prefix(
        pallet_gear_messenger::Pallet::<Runtime>::name().as_bytes(),
        "Head".as_bytes(),
    );
    // Ensure message queue is empty given the terminal extrinsic completed successfully
    assert!(state.storage(&queue_head_key[..]).unwrap().is_none());
}

#[test]
fn queue_remains_intact_if_processing_fails() {
    use sp_state_machine::IterArgs;

    init_logger();

    init!(client, backend, txpool, spawner, genesis_hash);

    // Create an extrinsic that prefunds the bank account
    let pre_fund_bank_xt = CheckedExtrinsic {
        signed: Some((alice(), signed_extra(0))),
        function: CallBuilder::deposit_to_bank().build(),
    };

    let mut checked = vec![pre_fund_bank_xt];
    checked.extend(checked_extrinsics(5, bob(), 0_u32, || {
        CallBuilder::noop().build()
    }));
    let nonce = 5_u32; // Bob's nonce for the future

    // Disable queue processing in Gear pallet as the root
    checked.push(CheckedExtrinsic {
        signed: Some((alice(), signed_extra(1))),
        function: CallBuilder::toggle_run_queue(false).build(),
    });
    let extrinsics = sign_extrinsics(
        checked,
        VERSION.spec_version,
        VERSION.transaction_version,
        genesis_hash,
    );
    submit_txs!(client, txpool, BlockId::number(0), extrinsics);

    let timestamp = Timestamp::current();

    propose_block!(
        client,
        backend,
        txpool,
        spawner,
        best_hash,
        BlockId::number(0),
        Box::new(time::Instant::now),
        timestamp,
        1500_u64,
        None,
        proposal
    );
    // Terminal extrinsic rolled back, therefore only have 1 inherent + 6 normal
    assert_eq!(proposal.block.extrinsics().len(), 8);

    // Ensure message queue still has 5 messages
    let state = backend.state_at(best_hash).unwrap();
    let queue_entry_prefix = storage_prefix(
        pallet_gear_messenger::Pallet::<Runtime>::name().as_bytes(),
        "Dispatches".as_bytes(),
    );
    let mut queue_entry_args = IterArgs::default();
    queue_entry_args.prefix = Some(&queue_entry_prefix);

    let mut queue_len = 0_u32;

    state
        .keys(queue_entry_args)
        .unwrap()
        .for_each(|_k| queue_len += 1);
    assert_eq!(queue_len, 5);

    // Preparing block #2
    let extrinsics = sign_extrinsics(
        checked_extrinsics(3, bob(), nonce, || CallBuilder::noop().build()),
        VERSION.spec_version,
        VERSION.transaction_version,
        genesis_hash,
    );
    submit_txs!(client, txpool, BlockId::Hash(best_hash), extrinsics);

    propose_block!(
        client,
        backend,
        txpool,
        spawner,
        best_hash,
        BlockId::Hash(best_hash),
        Box::new(time::Instant::now),
        Timestamp::new(timestamp.as_millis() + SLOT_DURATION),
        1500_u64,
        None,
        proposal
    );
    // Terminal extrinsic rolled back, therefore only have 1 inherent + 3 normal
    assert_eq!(proposal.block.extrinsics().len(), 4);

    let state = backend.state_at(best_hash).unwrap();
    // Ensure message queue has not been drained and has now 8 messages
    let mut queue_len = 0_u32;
    let mut queue_entry_args = IterArgs::default();
    queue_entry_args.prefix = Some(&queue_entry_prefix);
    state
        .keys(queue_entry_args)
        .unwrap()
        .for_each(|_k| queue_len += 1);
    assert_eq!(queue_len, 8);
}

#[test]
fn block_max_gas_works() {
    use sp_state_machine::IterArgs;

    // Amount of gas burned in each block (even empty) by default
    const FIXED_BLOCK_GAS: u64 = 25_000_000;

    init_logger();

    init!(client, backend, txpool, spawner, genesis_hash);

    // Prepare block #1
    // Create an extrinsic that prefunds the bank account
    let extrinsics = vec![sign(
        CheckedExtrinsic {
            signed: Some((alice(), signed_extra(0))),
            function: CallBuilder::deposit_to_bank().build(),
        },
        VERSION.spec_version,
        VERSION.transaction_version,
        genesis_hash,
    )
    .into()];
    submit_txs!(client, txpool, BlockId::number(0), extrinsics);

    let timestamp = Timestamp::current();
    propose_block!(
        client,
        backend,
        txpool,
        spawner,
        best_hash,
        BlockId::number(0),
        Box::new(time::Instant::now),
        timestamp,
        1500_u64,
        None,
        proposal
    );
    let api = client.runtime_api();
    let gear_core::gas::GasInfo { min_limit, .. } = api
        .calculate_gas_info(
            best_hash,
            sp_core::H256::from(alice().as_ref()),
            pallet_gear::HandleKind::Init(WASM_BINARY.to_vec()),
            Scheme::direct(Calls::builder().noop()).encode(),
            0,
            true,
            None,
            None,
        )
        .unwrap()
        .unwrap();

    // Just enough to fit 2 messages
    let max_gas = Some(2 * min_limit + FIXED_BLOCK_GAS + 100);

    // Preparing block #2
    // Creating 5 extrinsics
    let checked = checked_extrinsics(5, bob(), 0, || CallBuilder::noop().build());
    let extrinsics = sign_extrinsics(
        checked,
        VERSION.spec_version,
        VERSION.transaction_version,
        genesis_hash,
    );
    submit_txs!(client, txpool, BlockId::Hash(best_hash), extrinsics);

    let timestamp = Timestamp::new(timestamp.as_millis() + SLOT_DURATION);

    propose_block!(
        client,
        backend,
        txpool,
        spawner,
        best_hash,
        BlockId::Hash(best_hash),
        Box::new(time::Instant::now),
        Timestamp::new(timestamp.as_millis() + SLOT_DURATION),
        1500_u64,
        max_gas,
        proposal
    );
    // All extrinsics have been included in the block: 1 inherent + 5 normal + 1 terminal
    assert_eq!(proposal.block.extrinsics().len(), 7);

    let state = backend.state_at(best_hash).unwrap();
    // Ensure message queue still has 5 messages as none of the messages fit into the gas allownce
    let queue_entry_prefix = storage_prefix(
        pallet_gear_messenger::Pallet::<Runtime>::name().as_bytes(),
        "Dispatches".as_bytes(),
    );
    let mut queue_entry_args = IterArgs::default();
    queue_entry_args.prefix = Some(&queue_entry_prefix);

    let queue_len = state.keys(queue_entry_args).unwrap().count();

    // 2 out of 5 messages have been processed, 3 remain in the queue
    assert_eq!(queue_len, 3);

    let programs_prefix = storage_prefix(
        pallet_gear_program::Pallet::<Runtime>::name().as_bytes(),
        "ProgramStorage".as_bytes(),
    );
    let mut iter_args = IterArgs::default();
    iter_args.prefix = Some(&programs_prefix);

    // The fact that 2 init messages out of 5 have been processed means
    // that there should be 2 inited programs.
    let inited_count = state.pairs(iter_args).unwrap().fold(0u32, |count, pair| {
        let value = match pair {
            Ok((_key, value)) => value,
            _ => return count,
        };

        match Program::<BlockNumber>::decode(&mut &value[..]) {
            Ok(p) if p.is_initialized() => count + 1,
            _ => count,
        }
    });
    assert_eq!(inited_count, 2);
}

#[test]
fn terminal_extrinsic_discarded_from_txpool() {
    init_logger();

    init!(client, backend, txpool, spawner, genesis_hash);

    // Create Gear::run() extrinsic - both unsigned and signed
    let unsigned_gear_run_xt =
        UncheckedExtrinsic::new_unsigned(RuntimeCall::Gear(pallet_gear::Call::run {
            max_gas: None,
        }));
    let signed_gear_run_xt = sign(
        CheckedExtrinsic {
            signed: Some((bob(), signed_extra(0))),
            function: RuntimeCall::Gear(pallet_gear::Call::run { max_gas: None }),
        },
        VERSION.spec_version,
        VERSION.transaction_version,
        genesis_hash,
    );
    // A `DispatchClass::Normal` exrinsic - supposed to end up in the txpool
    let legit_xt = sign(
        CheckedExtrinsic {
            signed: Some((alice(), signed_extra(0))),
            function: CallBuilder::deposit_to_bank().build(),
        },
        VERSION.spec_version,
        VERSION.transaction_version,
        genesis_hash,
    );

    let extrinsics = vec![
        unsigned_gear_run_xt.into(),
        signed_gear_run_xt.into(),
        legit_xt.into(),
    ];
    submit_txs!(client, txpool, BlockId::number(0), extrinsics);
    assert_eq!(txpool.ready().count(), 1);

    propose_block!(
        client,
        backend,
        txpool,
        spawner,
        best_hash,
        BlockId::number(0),
        Box::new(time::Instant::now),
        Timestamp::current(),
        1500_u64,
        None,
        proposal
    );
    // Both mandatory extrinsics should have been discarded, therefore there are only 3 txs
    // in the block: 1 timestamp inherent + 1 normal extrinsic + 1 terminal
    assert_eq!(proposal.block.extrinsics().len(), 3);
}

#[test]
fn block_builder_cloned_ok() {
    init_logger();

    let client_builder =
        TestClientBuilder::new().set_execution_strategy(ExecutionStrategy::NativeWhenPossible);
    let backend = client_builder.backend();
    let client = Arc::new(client_builder.build());

    let genesis_hash =
        <[u8; 32]>::try_from(&client.info().best_hash[..]).expect("H256 is a 32 byte type");

    let extrinsics = sign_extrinsics(
        checked_extrinsics(5, bob(), 0, || CallBuilder::noop().build()),
        VERSION.spec_version,
        VERSION.transaction_version,
        genesis_hash,
    );

    let mut block_builder = BlockBuilder::new(
        client.as_ref(),
        genesis_hash.into(),
        0_u32,
        false.into(),
        pre_digest(1, 0),
        backend.as_ref(),
    )
    .unwrap();

    extrinsics.into_iter().for_each(|xt: OpaqueExtrinsic| {
        assert_ok!(block_builder.push(xt));
    });

    assert_eq!(block_builder.extrinsics().len(), 5);

    // At this point the overlay wrapped in the `Api` instance has some changes
    let fresh_block_builder = BlockBuilder::new(
        client.as_ref(),
        genesis_hash.into(),
        0_u32,
        false.into(),
        pre_digest(1, 0),
        backend.as_ref(),
    )
    .unwrap();

    let cloned_block_builder = block_builder.clone();

    let (ext_1, api_1, ver_1, phash_1, bd_1, hsize_1) = block_builder.deconstruct();
    let (ext_2, api_2, ver_2, phash_2, bd_2, hsize_2) = cloned_block_builder.deconstruct();

    // Assert that the components are equal but different
    assert_eq!(ext_1, ext_2);
    assert_ne!(ext_1.as_ptr(), ext_2.as_ptr());
    let api_1_ptr: *const <RA as sp_api::ConstructRuntimeApi<_, Client<_, _, _, RA>>>::RuntimeApi =
        api_1.deref();
    let api_2_ptr: *const <RA as sp_api::ConstructRuntimeApi<_, Client<_, _, _, RA>>>::RuntimeApi =
        api_2.deref();
    assert_ne!(api_1_ptr, api_2_ptr);

    // Reconstruct original block builders
    let block_builder = BlockBuilder::<'_, _, Client<_, _, _, RA>, _>::from_parts(
        ext_1, api_1, ver_1, phash_1, bd_1, hsize_1,
    );
    let cloned_block_builder = BlockBuilder::<'_, _, Client<_, _, _, RA>, _>::from_parts(
        ext_2, api_2, ver_2, phash_2, bd_2, hsize_2,
    );

    let changes_1 = block_builder.into_storage_changes().unwrap();
    let changes_2 = cloned_block_builder.into_storage_changes().unwrap();
    let changes_3 = fresh_block_builder.into_storage_changes().unwrap();

    // Assert that the original and the cloned block builders produce same storage changes
    assert_eq!(
        changes_1.transaction_storage_root,
        changes_2.transaction_storage_root
    );
    // that are different from what builder created from scratch produces
    assert_ne!(
        changes_1.transaction_storage_root,
        changes_3.transaction_storage_root
    );
}

#[test]
fn proposal_timing_consistent() {
    use sp_state_machine::IterArgs;

    init_logger();

    init!(client, backend, txpool, spawner, genesis_hash);

    // Create an extrinsic that prefunds the bank account
    let pre_fund_bank_xt = CheckedExtrinsic {
        signed: Some((alice(), signed_extra(0))),
        function: CallBuilder::deposit_to_bank().build(),
    };
    let mut checked = vec![pre_fund_bank_xt];

    // Creating a bunch of extrinsics to use up the quota for txpool processing
    // so that about 100 time-consuming init messages should end up in the queue.
    // It's possible though that not all of them make it into the block - it can depend on a
    // number of factors (timer on the target machine, log level, etc).
    checked.extend(checked_extrinsics(100, bob(), 0, || {
        // TODO: this is a "hand-wavy" workaround to have a long-running init message.
        // Should be replaced with a more reliable solution (like zero-cost syscalls
        // in init message that would guarantee incorrect gas estimation)
        CallBuilder::long_init(500_u64).build()
    }));
    let extrinsics = sign_extrinsics(
        checked,
        VERSION.spec_version,
        VERSION.transaction_version,
        genesis_hash,
    );

    submit_txs!(client, txpool, BlockId::number(0), extrinsics);
    let num_ready_1 = txpool.ready().count();

    let timestamp = Timestamp::current();

    // Simulate the situation when the `Gear::run()` takes longer time to execute than forecasted
    // (for instance, if the gas metering is not quite precise etc.) by setting the deadline to a
    // smaller value than in reality. On Vara the `max_duration` is 1.5s (which is then transformed
    // into 1s inside the `Proposer` and corresponds to 10^12 `max_block` weight).
    // Here we set it to 0.25s to try hit the timeout during the queue processing.
    let max_duration = 250_u64;

    propose_block!(
        client,
        backend,
        txpool,
        spawner,
        best_hash,
        BlockId::number(0),
        Box::new(time::Instant::now),
        timestamp,
        max_duration,
        None,
        proposal
    );

    let state = backend.state_at(best_hash).unwrap();

    // Check that the message queue has all messages pushed to it
    let queue_entry_prefix = storage_prefix(
        pallet_gear_messenger::Pallet::<Runtime>::name().as_bytes(),
        "Dispatches".as_bytes(),
    );
    let mut queue_entry_args = IterArgs::default();
    queue_entry_args.prefix = Some(&queue_entry_prefix);

    let queue_len = state.keys(queue_entry_args).unwrap().count();

    // Draining tx pool in preparation for block #2
    submit_txs!(client, txpool, BlockId::Hash(best_hash), vec![]);

    let num_ready_2 = txpool.ready().count();

    // `-1` for the bank account pre-funding which did't put anything in the queue.
    let num_messages = num_ready_1 - num_ready_2 - 1;

    // We expect the `Gear::run()` to have been dropped, hence the queue should
    // still have all the messages originally pushed to it.
    assert_eq!(queue_len, num_messages);

    // Let the `Gear::run()` thread a little more time to finish
    std::thread::sleep(time::Duration::from_millis(500));

    // In the meantime make sure we can still keep creating blocks
    // This time we set the deadline to a very high value to ensure that all messages go through.
    let max_duration = 15_000_u64;

    propose_block!(
        client,
        backend,
        txpool,
        spawner,
        best_hash,
        BlockId::Hash(best_hash),
        Box::new(time::Instant::now),
        Timestamp::new(timestamp.as_millis() + SLOT_DURATION),
        max_duration,
        None,
        proposal
    );
    // Importing block #2
    block_on(client.import(BlockOrigin::Own, proposal.block.clone())).unwrap();

    let state = backend.state_at(best_hash).unwrap();

    let queue_entry_prefix = storage_prefix(
        pallet_gear_messenger::Pallet::<Runtime>::name().as_bytes(),
        "Dispatches".as_bytes(),
    );
    let mut queue_entry_args = IterArgs::default();
    queue_entry_args.prefix = Some(&queue_entry_prefix);

    let queue_len = state.keys(queue_entry_args).unwrap().count();
    assert_eq!(queue_len, 0);
}

// Original tests from Substrate's `sc-basic-authorship` crate adjusted for actual Vara runtime
mod basic_tests {
    use super::*;

    use parking_lot::Mutex;

    fn extrinsic<E>(nonce: u32, signer: &AccountId, genesis_hash: [u8; 32]) -> E
    where
        E: From<UncheckedExtrinsic> + Clone,
    {
        sign_extrinsics::<E>(
            checked_extrinsics(1, signer.clone(), nonce, || CallBuilder::noop().build()),
            VERSION.spec_version,
            VERSION.transaction_version,
            genesis_hash,
        )[0]
        .clone()
    }

    fn exhausts_resources_extrinsic<E>(nonce: u32, signer: &AccountId, genesis_hash: [u8; 32]) -> E
    where
        E: From<UncheckedExtrinsic> + Clone,
    {
        sign_extrinsics::<E>(
            checked_extrinsics(1, signer.clone(), nonce, || {
                CallBuilder::exhaust_resources().build()
            }),
            VERSION.spec_version,
            VERSION.transaction_version,
            genesis_hash,
        )[0]
        .clone()
    }

    #[test]
    fn should_cease_building_block_when_deadline_is_reached() {
        init_logger();

        // given
        init!(client, backend, txpool, spawner, genesis_hash);

        let extrinsics = sign_extrinsics(
            checked_extrinsics(2, bob(), 0_u32, || CallBuilder::noop().build()),
            VERSION.spec_version,
            VERSION.transaction_version,
            genesis_hash,
        );
        submit_txs!(client, txpool, BlockId::number(0), extrinsics);

        let cell = Mutex::new((false, time::Instant::now()));

        // Proposer's `self.now()` function increments the `Instant` by 1s each time it's called
        // (starting from the moment of the second call)
        let now = Box::new(move || {
            let mut value = cell.lock();
            if !value.0 {
                value.0 = true;
                return value.1;
            }
            let old = value.1;
            let new = old + time::Duration::from_secs(1);
            *value = (true, new);
            old
        });

        // `max_duration` of 3s will be converted into 2s dealine inside `propose()`,
        // which allows the `self.now()` to be called 4 times before `deadline` is reached`:
        // - in `proposer::propose()` (1st time)
        // - after the inherents processing (2nd time)
        // - to limit the waiting time of the txpool (3rd time)
        // - at the 1st iteration of the tx processing `loop` (4th time)
        let max_duration = 3000_u64;

        propose_block!(
            client,
            backend,
            txpool,
            spawner,
            best_hash,
            BlockId::number(0),
            now,
            Timestamp::current(),
            max_duration,
            None,
            proposal
        );

        // then
        // The block should have 2 extrinsics: the timestamp and one ordinary one.
        // However, there is a tiny chance the `pseudo-inherent` can make it into the block
        // before the future measuring the timeout gets a chance to run.
        // We must account for the latter case, as well, to avoid ambiguity.
        if proposal.block.extrinsics().len() > 2 {
            assert_eq!(proposal.block.extrinsics().len(), 3);
            // A "hacky" way to ensure the unaccounted extrinsic is the pseudo-inherent:
            // its (current) encoded representation is `vec![16, 4, 104, 6, 0]`
            assert_eq!(
                proposal.block.extrinsics()[2].encode(),
                vec![16_u8, 4, 104, 6, 0]
            );
        } else {
            assert_eq!(proposal.block.extrinsics().len(), 2);
        }
        assert_eq!(txpool.ready().count(), 2);
    }

    #[test]
    fn should_not_panic_when_deadline_is_reached() {
        init_logger();

        init!(client, backend, txpool, spawner, _genesis_hash);

        let block_id = BlockId::number(0);
        let cell = Mutex::new((false, time::Instant::now()));
        // The `proposer.now()` that increments the `Instant` by 160s each time it's called
        let now = Box::new(move || {
            let mut value = cell.lock();
            if !value.0 {
                value.0 = true;
                return value.1;
            }
            let new = value.1 + time::Duration::from_secs(160);
            *value = (true, new);
            new
        });
        let max_duration = 1000_u64; // 1s

        propose_block!(
            client,
            backend,
            txpool,
            spawner,
            best_hash,
            block_id,
            now,
            Timestamp::current(),
            max_duration,
            None,
            proposal
        );
    }

    #[test]
    fn proposed_storage_changes_should_match_execute_block_storage_changes() {
        init_logger();

        init!(client, backend, txpool, spawner, genesis_hash);

        let extrinsics = sign_extrinsics(
            checked_extrinsics(1, bob(), 0_u32, || CallBuilder::noop().build()),
            VERSION.spec_version,
            VERSION.transaction_version,
            genesis_hash,
        );

        submit_txs!(client, txpool, BlockId::number(0), extrinsics);

        propose_block!(
            client,
            backend,
            txpool,
            spawner,
            best_hash,
            BlockId::number(0),
            Box::new(time::Instant::now),
            Timestamp::current(),
            1500_u64,
            None,
            proposal
        );
        // then
        // 1 inherent + 1 signed extrinsic + 1 terminal unsigned one
        assert_eq!(proposal.block.extrinsics().len(), 3);

        let api = client.runtime_api();
        api.execute_block(genesis_hash.into(), proposal.block)
            .unwrap();
        let state = backend.state_at(genesis_hash.into()).unwrap();

        let storage_changes = api.into_storage_changes(&state, best_hash).unwrap();

        assert_eq!(
            proposal.storage_changes.transaction_storage_root,
            storage_changes.transaction_storage_root,
        );

        let queue_head_key = storage_prefix(
            pallet_gear_messenger::Pallet::<Runtime>::name().as_bytes(),
            "Head".as_bytes(),
        );
        // Ensure message queue is empty given the terminal extrinsic completed successfully
        assert!(state.storage(&queue_head_key[..]).unwrap().is_none());
    }

    #[test]
    fn should_not_remove_invalid_transactions_when_skipping() {
        init_logger();

        // given
        init!(client, backend, txpool, spawner, genesis_hash);

        let alice = alice();

        let extrinsics = vec![
            extrinsic(0, &alice, genesis_hash),
            extrinsic(1, &alice, genesis_hash),
            exhausts_resources_extrinsic(2, &alice, genesis_hash),
            extrinsic(3, &alice, genesis_hash),
            exhausts_resources_extrinsic(4, &alice, genesis_hash),
            extrinsic(5, &alice, genesis_hash),
            extrinsic(6, &alice, genesis_hash),
        ];

        submit_txs!(client, txpool, BlockId::number(0), extrinsics);
        assert_eq!(txpool.ready().count(), 7);

        let timestamp = Timestamp::current();

        propose_block!(
            client,
            backend,
            txpool,
            spawner,
            best_hash,
            BlockId::number(0),
            Box::new(time::Instant::now),
            timestamp,
            1500_u64,
            None,
            proposal
        );
        // then
        // block should have some extrinsics although we have some more in the pool.
        assert_eq!(txpool.ready().count(), 7);
        assert_eq!(proposal.block.extrinsics().len(), 6);

        // Preparing block #2
        submit_txs!(client, txpool, BlockId::Hash(best_hash), vec![]);
        assert_eq!(txpool.ready().count(), 3);

        propose_block!(
            client,
            backend,
            txpool,
            spawner,
            best_hash,
            BlockId::Hash(best_hash),
            Box::new(time::Instant::now),
            Timestamp::new(timestamp.as_millis() + SLOT_DURATION),
            1500_u64,
            None,
            proposal
        );
        // 1 normal extrinsic should still make it into block (together with inherents):
        assert_eq!(txpool.ready().count(), 3);
        assert_eq!(proposal.block.extrinsics().len(), 5);
    }

    #[test]
    fn should_cease_building_block_when_block_limit_is_reached() {
        init_logger();

        init!(client, backend, txpool, spawner, genesis_hash);

        let block_id = BlockId::number(0);
        let genesis_header = client
            .header(client.block_hash_from_id(&block_id).unwrap().unwrap())
            .expect("header get error")
            .expect("there should be header");

        let extrinsics_num = 5_usize;
        let extrinsics = sign_extrinsics(
            checked_extrinsics(extrinsics_num as u32, bob(), 0_u32, || {
                CallBuilder::noop().build()
            }),
            VERSION.spec_version,
            VERSION.transaction_version,
            genesis_hash,
        );

        let timestamp_extrinsic_size = 11_usize;
        let tolerance = Perbill::from_percent(20);
        let all_but_extrinsics = (genesis_header.encoded_size()
            + Vec::<OpaqueExtrinsic>::new().encoded_size()
            + timestamp_extrinsic_size) as u32;
        let block_limit = (all_but_extrinsics + tolerance * all_but_extrinsics) as usize
            + extrinsics
                .iter()
                .take(extrinsics_num - 1)
                .map(Encode::encoded_size)
                .sum::<usize>();

        block_on(txpool.submit_at(&BlockId::number(0), SOURCE, extrinsics)).unwrap();

        block_on(txpool.maintain(chain_event(genesis_header.clone())));

        let mut proposer_factory = ProposerFactory::new(
            spawner.clone(),
            client.clone(),
            backend.clone(),
            txpool.clone(),
            None,
            None,
            None,
        );

        let proposer = block_on(proposer_factory.init(&genesis_header)).unwrap();

        // Give it enough time
        let deadline = time::Duration::from_secs(300_000);
        let timestamp = Timestamp::current();
        let timestamp_provider = sp_timestamp::InherentDataProvider::new(timestamp);
        let time_slot = timestamp.as_millis() / SLOT_DURATION;
        let inherent_data = block_on(timestamp_provider.create_inherent_data())
            .expect("Create inherent data failed");

        let block = block_on(proposer.propose(
            inherent_data.clone(),
            pre_digest(time_slot, 0),
            deadline,
            Some(block_limit),
        ))
        .map(|r| r.block)
        .unwrap();

        // Based on the block limit, one transaction shouldn't be included.
        // Instead, we have the timestamp and the pseudo-inherent.
        assert_eq!(block.extrinsics().len(), extrinsics_num - 1 + 2);

        let proposer = block_on(proposer_factory.init(&genesis_header)).unwrap();

        let block = block_on(proposer.propose(
            inherent_data.clone(),
            pre_digest(time_slot, 0),
            deadline,
            None,
        ))
        .map(|r| r.block)
        .unwrap();

        // Without a block limit we should include all of them + inherents
        assert_eq!(block.extrinsics().len(), extrinsics_num + 2);

        let mut proposer_factory = ProposerFactory::with_proof_recording(
            spawner.clone(),
            client.clone(),
            backend.clone(),
            txpool.clone(),
            None,
            None,
            None,
        );

        let proposer = block_on(proposer_factory.init(&genesis_header)).unwrap();

        // Give it enough time
        let block = block_on(proposer.propose(
            inherent_data,
            pre_digest(time_slot, 0),
            deadline,
            Some(block_limit),
        ))
        .map(|r| r.block)
        .unwrap();

        // The block limit didn't changed, but we now include the proof in the estimation of the
        // block size and thus, we fit in the block one ordinary extrinsic less as opposed to
        // `extrinsics_num - 1` extrinsics we could fit earlier (mind the inherents, as usually).
        assert_eq!(block.extrinsics().len(), extrinsics_num - 2 + 2);
    }

    #[test]
    fn should_keep_adding_transactions_after_exhausts_resources_before_soft_deadline() {
        // given
        init_logger();

        // given
        init!(client, backend, txpool, spawner, genesis_hash);

        let alice = alice();
        let bob = bob();

        let extrinsics = (0_usize..MAX_SKIPPED_TRANSACTIONS * 2)
            .map(|i| exhausts_resources_extrinsic(i as u32, &alice, genesis_hash))
            // and some transactions that are okay.
            .chain(
                (0_usize..MAX_SKIPPED_TRANSACTIONS)
                    .map(|i| extrinsic(i as u32, &bob, genesis_hash)),
            )
            .collect();

        submit_txs!(client, txpool, BlockId::number(0), extrinsics);
        assert_eq!(txpool.ready().count(), MAX_SKIPPED_TRANSACTIONS * 3);

        // give it enough time so that deadline is never triggered.
        let max_duration = 900_000_u64;

        propose_block!(
            client,
            backend,
            txpool,
            spawner,
            best_hash,
            BlockId::number(0),
            Box::new(time::Instant::now),
            Timestamp::current(),
            max_duration,
            None,
            proposal
        );
        // then
        // MAX_SKIPPED_TRANSACTIONS + inherents have been included in the block
        assert_eq!(
            proposal.block.extrinsics().len(),
            MAX_SKIPPED_TRANSACTIONS + 3
        );
    }

    #[test]
    fn should_only_skip_up_to_some_limit_after_soft_deadline() {
        // given
        init_logger();

        init!(client, backend, txpool, spawner, genesis_hash);

        let bob = bob();
        let extrinsics = (0_usize..MAX_SKIPPED_TRANSACTIONS + 2)
            .map(|i| exhausts_resources_extrinsic(i as u32, &bob, genesis_hash))
            // and some transactions that are okay.
            .chain(
                (MAX_SKIPPED_TRANSACTIONS + 2..2_usize * MAX_SKIPPED_TRANSACTIONS + 2)
                    .map(|i| extrinsic(i as u32, &bob, genesis_hash)),
            )
            .collect();
        let block_id = BlockId::number(0);

        submit_txs!(client, txpool, block_id, extrinsics);
        assert_eq!(txpool.ready().count(), MAX_SKIPPED_TRANSACTIONS * 2 + 2);

        let cell = Arc::new(Mutex::new((0, time::Instant::now())));
        let cell2 = cell.clone();
        let max_duration = 1_000_000_u64; // more than enough time
        let now = Box::new(move || {
            let mut value = cell.lock();
            let (called, old) = *value;
            // add time after deadline is calculated internally (hence 1)
            let increase = if called == 1 {
                // we start after the soft_deadline should have already been reached.
                time::Duration::from_millis(max_duration) / 2
            } else {
                // but we make sure to never reach the actual deadline
                time::Duration::from_millis(0)
            };
            *value = (called + 1, old + increase);
            old
        });

        propose_block!(
            client,
            backend,
            txpool,
            spawner,
            best_hash,
            block_id,
            now,
            Timestamp::current(),
            max_duration,
            None,
            proposal
        );
        // the block should have a single ordinary transaction despite more being in the pool
        assert_eq!(proposal.block.extrinsics().len(), 3);
        assert!(
            cell2.lock().0 > MAX_SKIPPED_TRANSACTIONS,
            "Not enough calls to current time, which indicates the test might have ended \
            because of deadline, not soft deadline"
        );
    }
}
