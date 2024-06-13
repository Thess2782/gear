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

use crate::{host::InstanceWrapper, Database, UserMessage};
use core_processor::common::JournalNote;
use gear_core::{
    ids::{ActorId, ProgramId},
    message::{Message, Payload},
};
use hypercore_runtime_common::{
    state::{Dispatch, MaybeHash, ProgramState, Storage},
    Handler,
};
use primitive_types::H256;
use std::collections::BTreeMap;
use tokio::sync::{mpsc, oneshot};

struct Task {
    data: (ProgramId, H256),
    result_sender: oneshot::Sender<Vec<JournalNote>>,
}

pub fn run(
    threads_amount: usize,
    db: Database,
    programs: &mut BTreeMap<ProgramId, H256>,
    messages: BTreeMap<ProgramId, Vec<UserMessage>>,
) -> Vec<Message> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(threads_amount)
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async { run_in_async(db, programs, messages).await })
}

// TODO: Returning Vec<Message> is a temporary solution.
// In future need to send all messages to users and all state hashes changes to sequencer.
async fn run_in_async(
    db: Database,
    programs: &mut BTreeMap<ProgramId, H256>,
    messages: BTreeMap<ProgramId, Vec<UserMessage>>,
) -> Vec<Message> {
    let mut to_users_messages = vec![];

    update_queues(&db, programs, messages);

    let num_workers = 4;

    let mut task_senders = vec![];
    let mut handles = vec![];

    // create workers
    for id in 0..num_workers {
        let (task_sender, task_receiver) = mpsc::channel(100);
        task_senders.push(task_sender);
        let handle = tokio::spawn(worker(id, db.clone(), task_receiver));
        handles.push(handle);
    }

    loop {
        // Send tasks to process programs in workers, until all queues are empty.

        let mut no_more_to_do = true;
        for index in (0..programs.len()).step_by(num_workers) {
            let result_receivers = one_batch(index, &task_senders, programs).await;

            let mut super_journal = vec![];
            for (program_id, receiver) in result_receivers.into_iter() {
                let journal = receiver.await.unwrap();
                if !journal.is_empty() {
                    no_more_to_do = false;
                }
                super_journal.push((program_id, journal));
            }

            for (program_id, journal) in super_journal {
                let mut handler = Handler {
                    program_id,
                    program_states: programs,
                    storage: &db,
                    block_info: Default::default(),
                    to_users_messages: Default::default(),
                };
                core_processor::handle_journal(journal, &mut handler);
                to_users_messages.append(&mut handler.to_users_messages);
            }
        }

        if no_more_to_do {
            break;
        }
    }

    for handle in handles {
        handle.abort();
    }

    to_users_messages
}

async fn run_task(db: Database, instance: &mut InstanceWrapper, task: Task) {
    let (program_id, state_hash) = task.data;

    let code_id = db
        .get_program_code_id(program_id)
        .expect("Code ID must be set");

    let instrumented_code = db.read_instrumented_code(hypercore_runtime::VERSION, code_id);

    let journal = instance
        .run(program_id, code_id, state_hash, instrumented_code)
        .expect("Some error occurs while running program in instance");

    task.result_sender.send(journal).unwrap();
}

async fn worker(_id: usize, db: Database, mut task_receiver: mpsc::Receiver<Task>) {
    let mut instance_wrapper =
        InstanceWrapper::new(db.clone()).expect("Cannot create runtime instance");
    while let Some(task) = task_receiver.recv().await {
        run_task(db.clone(), &mut instance_wrapper, task).await;
    }
}

async fn one_batch(
    from_index: usize,
    task_senders: &[mpsc::Sender<Task>],
    programs: &mut BTreeMap<ActorId, H256>,
) -> BTreeMap<ProgramId, oneshot::Receiver<Vec<JournalNote>>> {
    let mut result_receivers = BTreeMap::new();

    for (sender, (program_id, state_hash)) in
        task_senders.iter().zip(programs.iter().skip(from_index))
    {
        let (result_sender, result_receiver) = oneshot::channel();

        let task = Task {
            data: (*program_id, *state_hash),
            result_sender,
        };

        sender.send(task).await.unwrap();

        result_receivers.insert(*program_id, result_receiver);
    }

    result_receivers
}

fn update_queues(
    db: &Database,
    programs: &mut BTreeMap<ProgramId, H256>,
    mut messages: BTreeMap<ProgramId, Vec<UserMessage>>,
) {
    for (program_id, state_hash) in programs.iter_mut() {
        let state = db.read_state(*state_hash).unwrap();
        let mut queue = state
            .queue_hash
            .with_hash_or_default(|hash| db.read_queue(hash).unwrap_or_default());
        let messages = messages.remove(program_id).unwrap_or_default();
        for message in messages.into_iter() {
            let payload_hash = match message.payload.len() {
                0 => MaybeHash::Empty,
                _ => db
                    .write_payload(Payload::try_from(message.payload).unwrap())
                    .into(),
            };

            let dispatch = Dispatch {
                id: message.id,
                kind: message.kind,
                source: message.source,
                payload_hash,
                gas_limit: message.gas_limit,
                value: message.value,
                details: None,
                context: None,
            };

            queue.push_back(dispatch);
        }

        let mut waitlist = state
            .waitlist_hash
            .with_hash_or_default(|hash| db.read_waitlist(hash).unwrap_or_default());

        let mut dispatches_to_wake = Vec::new();
        let mut blocks_to_remove = Vec::new();
        for (block, list) in waitlist.range_mut(0..=0) {
            if list.is_empty() {
                unreachable!("Empty waitlist for block, must been removed from waitlist")
            }
            dispatches_to_wake.append(list);
            blocks_to_remove.push(*block);
        }

        for block in blocks_to_remove {
            waitlist.remove(&block);
        }

        for dispatch in dispatches_to_wake {
            queue.push_back(dispatch);
        }

        let queue_hash = db.write_queue(queue).into();
        let waitlist_hash = db.write_waitlist(waitlist).into();

        let state = ProgramState {
            queue_hash,
            waitlist_hash,
            ..state
        };

        *state_hash = db.write_state(state);
    }
}
