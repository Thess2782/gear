// This file is part of Gear.

// Copyright (C) 2021-2024 Gear Technologies Inc.
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

use crate::{
    log::{BlockRunResult, CoreLog},
    manager::ExtManager,
    program::{Program, ProgramIdWrapper},
    state::{accounts::Accounts, actors::Actors, mailbox::ActorMailbox},
    Gas, Value, GAS_ALLOWANCE,
};
use codec::{Decode, DecodeAll};
use colored::Colorize;
use env_logger::{Builder, Env};
use gear_core::{
    ids::{CodeId, ProgramId},
    pages::GearPage,
};
use gear_lazy_pages::{LazyPagesStorage, LazyPagesVersion};
use gear_lazy_pages_common::LazyPagesInitContext;
use path_clean::PathClean;
use std::{borrow::Cow, cell::RefCell, env, fs, io::Write, mem, path::Path, thread};

thread_local! {
    /// `System` is a singleton with a one instance and no copies returned.
    ///
    /// `OnceCell` is used to control one-time instantiation, while `RefCell`
    /// is needed for interior mutability to uninitialize the global.
    static SYSTEM_INITIALIZED: RefCell<bool> = const { RefCell::new(false) };
}

#[derive(Decode)]
#[codec(crate = codec)]
struct PageKey {
    _page_storage_prefix: [u8; 32],
    program_id: ProgramId,
    _memory_infix: u32,
    page: GearPage,
}

#[derive(Debug)]
struct PagesStorage;

impl LazyPagesStorage for PagesStorage {
    fn page_exists(&self, mut key: &[u8]) -> bool {
        let PageKey {
            program_id, page, ..
        } = PageKey::decode_all(&mut key).expect("Invalid key");

        Actors::access(program_id, |actor| {
            actor
                .and_then(|actor| actor.get_pages_data())
                .map(|pages_data| pages_data.contains_key(&page))
                .unwrap_or(false)
        })
    }

    fn load_page(&mut self, mut key: &[u8], buffer: &mut [u8]) -> Option<u32> {
        let PageKey {
            program_id, page, ..
        } = PageKey::decode_all(&mut key).expect("Invalid key");

        Actors::access(program_id, |actor| {
            actor
                .and_then(|actor| actor.get_pages_data())
                .and_then(|pages_data| pages_data.get(&page))
                .map(|page_buf| {
                    buffer.copy_from_slice(page_buf);
                    page_buf.len() as u32
                })
        })
    }
}

/// The testing environment which simulates the chain state and its
/// transactions but somehow the real on-chain execution environment
/// could be different.
///
/// ```
/// use gtest::System;
///
/// // Create a new testing environment.
/// let system = System::new();
///
/// // Init logger with "gwasm" target set to `debug` level.
/// system.init_logger();
/// ```
pub struct System(pub(crate) RefCell<ExtManager>);

impl System {
    /// Prefix for lazy pages.
    pub(crate) const PAGE_STORAGE_PREFIX: [u8; 32] = *b"gtestgtestgtestgtestgtestgtest00";

    /// Create a new testing environment.
    ///
    /// # Panics
    /// Only one instance in the current thread of the `System` is possible to
    /// create. Instantiation of the other one leads to runtime panic.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SYSTEM_INITIALIZED.with_borrow_mut(|initialized| {
            if *initialized {
                panic!("Impossible to have multiple instances of the `System`.");
            }

            let ext_manager = ExtManager::new();
            gear_lazy_pages::init(
                LazyPagesVersion::Version1,
                LazyPagesInitContext::new(Self::PAGE_STORAGE_PREFIX),
                PagesStorage,
            )
            .expect("Failed to init lazy-pages");

            *initialized = true;

            Self(RefCell::new(ext_manager))
        })
    }

    /// Init logger with "gwasm" target set to `debug` level.
    pub fn init_logger(&self) {
        self.init_logger_with_default_filter("gwasm=debug");
    }

    /// Init logger with "gwasm" and "gtest" targets set to `debug` level.
    pub fn init_verbose_logger(&self) {
        self.init_logger_with_default_filter("gwasm=debug,gtest=debug");
    }

    /// Init logger with `default_filter` as default filter.
    pub fn init_logger_with_default_filter<'a>(&self, default_filter: impl Into<Cow<'a, str>>) {
        let _ = Builder::from_env(Env::default().default_filter_or(default_filter))
            .format(|buf, record| {
                let lvl = record.level().to_string().to_uppercase();
                let target = record.target().to_string();
                let mut msg = record.args().to_string();

                if target == "gwasm" {
                    msg = msg.replacen("DEBUG: ", "", 1);

                    writeln!(
                        buf,
                        "[{} {}] {}",
                        lvl.blue(),
                        thread::current().name().unwrap_or("unknown").white(),
                        msg.white()
                    )
                } else {
                    writeln!(
                        buf,
                        "[{} {}] {}",
                        target.red(),
                        thread::current().name().unwrap_or("unknown").white(),
                        msg.white()
                    )
                }
            })
            .format_target(false)
            .format_timestamp(None)
            .try_init();
    }

    /// Run next block.
    ///
    /// Block execution model is the following:
    /// - increase the block number, update the timestamp
    /// - process tasks from the task pool
    /// - process messages in the queue.
    ///
    /// The system is always initialized with a 0 block number. Current block
    /// number in the system is the number of the already executed block,
    /// therefore block execution starts with a block info update (block
    /// number increase, timestamp update). For example, if current block
    /// number is 2, it means that messages and tasks on 2 were executed, so
    /// the method goes to block number 3 and executes tasks and messages for
    /// the updated block number.
    ///
    /// Task processing basically tries to execute the scheduled to the specific
    /// block tasks:
    /// - delayed sending
    /// - waking message
    /// - removing from the mailbox
    /// - removing reservations
    /// - removing stalled wait message.
    ///
    /// Messages processing executes messages until either queue becomes empty
    /// or block gas allowance is fully consumed.
    pub fn run_next_block(&self) -> BlockRunResult {
        self.run_next_block_with_allowance(Gas(GAS_ALLOWANCE))
    }

    /// Runs blocks same as [`Self::run_next_block`], but with limited
    /// allowance.
    pub fn run_next_block_with_allowance(&self, allowance: Gas) -> BlockRunResult {
        if allowance > Gas(GAS_ALLOWANCE) {
            panic!("Provided allowance more than allowed limit of {GAS_ALLOWANCE}.");
        }

        self.0.borrow_mut().run_new_block(allowance)
    }

    /// Runs blocks same as [`Self::run_next_block`], but executes blocks to
    /// block number `bn` including it.
    pub fn run_to_block(&self, bn: u32) -> Vec<BlockRunResult> {
        let mut manager = self.0.borrow_mut();

        let mut current_block = manager.block_height();
        if current_block > bn {
            panic!("Can't run blocks until bn {bn}, as current bn is {current_block}");
        }

        let mut ret = Vec::with_capacity((bn - current_block) as usize);
        while current_block != bn {
            let res = manager.run_new_block(Gas(GAS_ALLOWANCE));
            ret.push(res);

            current_block = manager.block_height();
        }

        ret
    }

    /// Runs `amount` of blocks only with processing task pool, without
    /// processing the message queue.
    pub fn run_scheduled_tasks(&self, amount: u32) -> Vec<BlockRunResult> {
        let mut manager = self.0.borrow_mut();
        let block_height = manager.block_height();

        (block_height..block_height + amount)
            .map(|_| {
                manager.check_epoch();

                let block_info = manager.blocks_manager.next_block();
                let next_block_number = block_info.height;
                manager.process_tasks(next_block_number);

                let log = mem::take(&mut manager.log)
                    .into_iter()
                    .map(CoreLog::from)
                    .collect();
                BlockRunResult {
                    block_info,
                    gas_allowance_spent: Gas(GAS_ALLOWANCE) - manager.gas_allowance,
                    log,
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Return the current block height of the testing environment.
    pub fn block_height(&self) -> u32 {
        self.0.borrow().block_height()
    }

    /// Return the current block timestamp of the testing environment.
    pub fn block_timestamp(&self) -> u64 {
        self.0.borrow().blocks_manager.get().timestamp
    }

    /// Returns a [`Program`] by `id`.
    pub fn get_program<ID: Into<ProgramIdWrapper>>(&self, id: ID) -> Option<Program> {
        let id = id.into().0;
        if Actors::is_program(id) {
            Some(Program {
                id,
                manager: &self.0,
            })
        } else {
            None
        }
    }

    /// Returns last added program.
    pub fn last_program(&self) -> Option<Program> {
        self.programs().into_iter().next_back()
    }

    /// Returns a list of programs.
    pub fn programs(&self) -> Vec<Program> {
        Actors::program_ids()
            .into_iter()
            .map(|id| Program {
                id,
                manager: &self.0,
            })
            .collect()
    }

    /// Detect if a program is active with given `id`.
    ///
    /// An active program means that the program could be called,
    /// instead, if returns `false` it means that the program has
    /// exited or terminated that it can't be called anymore.
    pub fn is_active_program<ID: Into<ProgramIdWrapper>>(&self, id: ID) -> bool {
        let program_id = id.into().0;
        Actors::is_active_program(program_id)
    }

    /// Saves code to the storage and returns its code hash
    ///
    /// This method is mainly used for providing a proper program from program
    /// creation logic. In order to successfully create a new program with
    /// `gstd::prog::create_program_bytes_with_gas` function, developer should
    /// provide to the function "child's" code hash. Code for that code hash
    /// must be in storage at the time of the function call. So this method
    /// stores the code in storage.
    pub fn submit_code(&self, binary: impl Into<Vec<u8>>) -> CodeId {
        self.0.borrow_mut().store_new_code(binary.into())
    }

    /// Saves code from file to the storage and returns its code hash
    ///
    /// See also [`System::submit_code`]
    #[track_caller]
    pub fn submit_code_file<P: AsRef<Path>>(&self, code_path: P) -> CodeId {
        let code = fs::read(&code_path).unwrap_or_else(|_| {
            panic!(
                "Failed to read file {}",
                code_path.as_ref().to_string_lossy()
            )
        });
        self.0.borrow_mut().store_new_code(code)
    }

    /// Saves code to the storage and returns its code hash
    ///
    /// Same as ['submit_code_file'], but the path is provided as relative to
    /// the current directory.
    #[track_caller]
    pub fn submit_local_code_file<P: AsRef<Path>>(&self, code_path: P) -> CodeId {
        let path = env::current_dir()
            .expect("Unable to get root directory of the project")
            .join(code_path)
            .clean();

        self.submit_code_file(path)
    }

    /// Returns previously submitted code by its code hash.
    pub fn submitted_code(&self, code_id: CodeId) -> Option<Vec<u8>> {
        self.0.borrow().read_code(code_id).map(|code| code.to_vec())
    }

    /// Extract mailbox of user with given `id`.
    ///
    /// The mailbox contains messages from the program that are waiting
    /// for user action.
    #[track_caller]
    pub fn get_mailbox<ID: Into<ProgramIdWrapper>>(&self, id: ID) -> ActorMailbox {
        let program_id = id.into().0;
        if !Actors::is_user(program_id) {
            panic!("Mailbox available only for users");
        }
        ActorMailbox::new(program_id, &self.0)
    }

    /// Mint balance to user with given `id` and `value`.
    pub fn mint_to<ID: Into<ProgramIdWrapper>>(&self, id: ID, value: Value) {
        let id = id.into().0;

        if Actors::is_program(id) {
            panic!(
                "Attempt to mint value to a program {id:?}, please use `System::transfer` instead"
            );
        }

        self.0.borrow_mut().mint_to(&id, value);
    }

    /// Transfer balance from user with given `from` id to user with given `to`
    /// id.
    pub fn transfer(
        &self,
        from: impl Into<ProgramIdWrapper>,
        to: impl Into<ProgramIdWrapper>,
        value: Value,
        keep_alive: bool,
    ) {
        let from = from.into().0;
        let to = to.into().0;

        if Actors::is_program(from) {
            panic!("Attempt to transfer from a program {from:?}");
        }

        Accounts::transfer(from, to, value, keep_alive);
    }

    /// Returns balance of user with given `id`.
    pub fn balance_of<ID: Into<ProgramIdWrapper>>(&self, id: ID) -> Value {
        let actor_id = id.into().0;
        self.0.borrow().balance_of(&actor_id)
    }
}

impl Drop for System {
    fn drop(&mut self) {
        // Uninitialize
        SYSTEM_INITIALIZED.with_borrow_mut(|initialized| *initialized = false);
        self.0.borrow().gas_tree.reset();
        self.0.borrow().mailbox.reset();
        self.0.borrow().task_pool.clear();
        self.0.borrow().waitlist.reset();

        // Clear actors and accounts storages
        Actors::clear();
        Accounts::clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Impossible to have multiple instances of the `System`.")]
    fn test_system_being_singleton() {
        let _first_instance = System::new();

        let _second_instance = System::new();
    }

    #[test]
    fn test_multithread_copy_singleton() {
        let first_instance = System::new();
        first_instance.run_scheduled_tasks(5);

        assert_eq!(first_instance.block_height(), 5);

        let h = std::thread::spawn(|| {
            let second_instance = System::new();

            second_instance.run_scheduled_tasks(10);
            assert_eq!(second_instance.block_height(), 10);
        });

        h.join().expect("internal error failed joining thread");
    }

    #[test]
    fn test_bn_adjustments() {
        let sys = System::new();
        assert_eq!(sys.block_height(), 0);

        // ### Check block info after run to next block ###
        let res = sys.run_next_block();
        let block_info = res.block_info;
        assert_eq!(block_info.height, sys.block_height());
        assert_eq!(block_info.height, 1);

        // ### Check block info after run to block ###
        let current_height = block_info.height;
        let until_height = 5;
        let results = sys.run_to_block(until_height);
        assert_eq!(results.len(), (until_height - current_height) as usize);

        // Check first block executed is always the next block
        let first_run = results.first().expect("checked above");
        assert_eq!(first_run.block_info.height, current_height + 1);

        // Check the last block executed number
        let last_run = results.last().expect("checked above");
        assert_eq!(last_run.block_info.height, until_height);
        assert_eq!(last_run.block_info.height, sys.block_height());

        // ### Check block info after running the task pool ###
        let current_height = last_run.block_info.height;
        let amount_of_blocks = 10;
        let results = sys.run_scheduled_tasks(amount_of_blocks);
        assert_eq!(results.len(), amount_of_blocks as usize);

        let first_run = results.first().expect("checked above");
        assert_eq!(first_run.block_info.height, current_height + 1);

        let last_run = results.last().expect("checked above");
        assert_eq!(
            last_run.block_info.height,
            current_height + amount_of_blocks
        );

        assert_eq!(last_run.block_info.height, 15);
    }
}
