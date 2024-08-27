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

use crate::client::{Backend, Code, Message, Program, TxResult};
use anyhow::{anyhow, Result};
use gear_core::ids::ProgramId;
use gprimitives::{ActorId, MessageId, H256};
use gtest::System;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    time::{Duration, SystemTime},
};

/// gear general client gtest backend
#[derive(Clone)]
pub struct GTest {
    tx: Sender<Request>,
    resps: Arc<Mutex<HashMap<usize, Response>>>,
    timeout: Duration,
    nonce: Arc<AtomicUsize>,
}

const DEFAULT_TIMEOUT: u64 = 500;

impl GTest {
    /// Get gtest result from nonce.
    async fn resp(&self, nonce: usize) -> Result<Response> {
        let now = SystemTime::now();

        loop {
            if now.elapsed()? > self.timeout {
                return Err(anyhow!("gtest: Transaction timed out!"));
            }

            if let Ok(Some(resp)) = self.resps.lock().map(|mut resps| resps.remove(&nonce)) {
                return Ok(resp);
            }
        }
    }

    /// Spawn gtest service
    fn spawn(resps: Arc<Mutex<HashMap<usize, Response>>>, rx: Receiver<Request>) {
        std::thread::spawn(move || {
            let system = System::new();
            while let Ok(tx) = rx.recv() {
                let (result, nonce) = match tx {
                    Request::Deploy {
                        nonce,
                        code,
                        message,
                        signer,
                    } => (handle::deploy(&system, code, message, signer), nonce),
                    Request::Send {
                        nonce,
                        prog,
                        message,
                        signer,
                    } => (handle::send(&system, prog, message, signer), nonce),
                    Request::Program { nonce, id } => (handle::prog(&system, id), nonce),
                    Request::MintTo {
                        actor,
                        value,
                        nonce,
                    } => (handle::mint_to(&system, actor, value), nonce),
                };

                if let Ok(mut resps) = resps.lock() {
                    resps.insert(nonce, result);
                }
            }
        });
    }
}

impl Backend for GTest {
    async fn program(&self, id: ProgramId) -> Result<Program<Self>> {
        let nonce = self.nonce.load(Ordering::SeqCst);
        self.tx.send(Request::Program { nonce, id })?;

        let result = self.resp(nonce).await?;
        let Response::Program(result) = result else {
            return Err(anyhow!(
                "Response is not matched with deploy request, {result:?}"
            ));
        };

        Ok(Program {
            id: result.ok_or(anyhow!("Program {id} not found"))?,
            backend: self.clone(),
        })
    }

    async fn deploy<M>(&self, code: impl Code, message: M) -> Result<TxResult<Program<Self>>>
    where
        M: Into<Message> + Send,
    {
        let nonce = self.nonce.load(Ordering::SeqCst);
        self.tx.send(Request::Deploy {
            nonce,
            code: code.bytes()?,
            message: message.into(),
            signer: Default::default(),
        })?;

        let result = self.resp(nonce).await?;
        let Response::Deploy(result) = result else {
            return Err(anyhow!(
                "Response is not matched with deploy request, {result:?}"
            ));
        };

        Ok(TxResult {
            result: Program {
                id: result.result,
                backend: self.clone(),
            },
            logs: result.logs,
        })
    }

    async fn send<M>(&self, id: ProgramId, message: M) -> Result<TxResult<MessageId>>
    where
        M: Into<Message> + Send,
    {
        let nonce = self.nonce.load(Ordering::SeqCst);
        self.tx.send(Request::Send {
            nonce,
            prog: id,
            message: message.into(),
            signer: Default::default(),
        })?;

        let result = self.resp(nonce).await?;
        let Response::Send(result) = result else {
            return Err(anyhow!(
                "Response is not matched with sent request, {result:?}"
            ));
        };

        result.ensure()
    }

    async fn transfer(&self, to: ActorId, value: u128) -> Result<TxResult<H256>> {
        let nonce = self.nonce.load(Ordering::SeqCst);
        self.tx.send(Request::MintTo {
            actor: to,
            value,
            nonce,
        })?;

        let _ = self.resp(nonce).await?;

        Ok(TxResult {
            result: Default::default(),
            logs: Default::default(),
        })
    }

    fn timeout(&mut self, timeout: Duration) {
        self.timeout = timeout
    }
}

impl Default for GTest {
    fn default() -> Self {
        let resps = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = mpsc::channel::<Request>();
        Self::spawn(resps.clone(), rx);

        Self {
            tx,
            resps,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT),
            nonce: Arc::new(AtomicUsize::new(0)),
        }
    }
}

/// GTest requests
pub enum Request {
    Deploy {
        nonce: usize,
        code: Vec<u8>,
        message: Message,
        signer: ActorId,
    },
    Send {
        nonce: usize,
        prog: ActorId,
        message: Message,
        signer: ActorId,
    },
    Program {
        nonce: usize,
        id: ProgramId,
    },
    MintTo {
        nonce: usize,
        actor: ActorId,
        value: u128,
    },
}

/// GTest responses
#[derive(Debug)]
pub enum Response {
    Deploy(TxResult<ActorId>),
    Send(TxResult<Result<MessageId>>),
    Program(Option<ActorId>),
    MintTo,
}

/// gtest handles
pub(crate) mod handle {
    use crate::client::{backend::gtest::Response, Message, TxResult};
    use anyhow::anyhow;
    use gear_core::{
        buffer::LimitedVec,
        ids::{prelude::CodeIdExt, ProgramId},
        message::{ReplyDetails, UserMessage},
    };
    use gprimitives::{ActorId, CodeId};
    use gtest::{CoreLog, Program, System};

    /// Return back program id if program exists
    pub fn prog(system: &System, prog: ProgramId) -> Response {
        Response::Program(system.get_program(prog).map(|p| p.id()))
    }

    /// Mint value to actor id
    pub fn mint_to(system: &System, actor: ActorId, value: u128) -> Response {
        system.mint_to(actor, value);
        Response::MintTo
    }

    /// Deploy program via gtest
    pub fn deploy(system: &System, code: Vec<u8>, message: Message, signer: ActorId) -> Response {
        let id = CodeId::generate(&code);
        let prog = Program::from_binary_with_id(system, id.into_bytes().to_vec(), code);
        let r = prog.send_bytes(signer, message.payload);

        Response::Deploy(TxResult {
            result: prog.id(),
            logs: map_logs(r.log()),
        })
    }

    /// Send message via gtest
    pub fn send(system: &System, prog: ActorId, message: Message, signer: ActorId) -> Response {
        let Some(prog) = system.get_program(prog) else {
            return Response::Send(TxResult::error(anyhow!("program {prog} not found")));
        };

        let r = prog.send_bytes(signer, message.payload);
        Response::Send(TxResult {
            result: Ok(r.sent_message_id()),
            logs: map_logs(r.log()),
        })
    }

    fn map_logs(logs: &[CoreLog]) -> Vec<UserMessage> {
        logs.iter()
            .map(|l| {
                UserMessage::new(
                    l.id(),
                    l.source(),
                    l.destination(),
                    LimitedVec::try_from(l.payload().to_vec()).unwrap_or_default(),
                    Default::default(),
                    l.reply_code()
                        .zip(l.reply_to())
                        .map(|(code, to)| ReplyDetails::new(to, code)),
                )
            })
            .collect()
    }
}
