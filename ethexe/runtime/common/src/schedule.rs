use crate::{
    state::{Dispatch, Expiring, MailboxMessage, PayloadLookup, Storage, MAILBOX_VALIDITY},
    TransitionController,
};
use ethexe_common::{
    db::{Rfm, ScheduledTask, Sd, Sum},
    gear::ValueClaim,
};
use gear_core::{ids::ProgramId, tasks::TaskHandler};
use gear_core_errors::SuccessReplyReason;
use gprimitives::{ActorId, CodeId, MessageId, ReservationId};

pub struct Handler<'a, S: Storage> {
    pub controller: TransitionController<'a, S>,
}

impl<S: Storage> TaskHandler<Rfm, Sd, Sum> for Handler<'_, S> {
    fn remove_from_mailbox(
        &mut self,
        (program_id, user_id): (ProgramId, ActorId),
        message_id: MessageId,
    ) -> u64 {
        self.controller
            .update_state(program_id, |state, storage, transitions| {
                let Expiring {
                    value: MailboxMessage { value, origin, .. },
                    ..
                } = state.mailbox_hash.modify_mailbox(storage, |mailbox| {
                    mailbox
                        .remove_and_store_user_mailbox(storage, user_id, message_id)
                        .expect("failed to find message in mailbox")
                });

                transitions.modify_transition(program_id, |transition| {
                    transition.claims.push(ValueClaim {
                        message_id,
                        destination: user_id,
                        value,
                    })
                });

                let reply = Dispatch::reply(
                    message_id,
                    user_id,
                    PayloadLookup::empty(),
                    0,
                    SuccessReplyReason::Auto,
                    origin,
                );

                state
                    .queue_hash
                    .modify_queue(storage, |queue| queue.queue(reply));
            });

        0
    }

    fn send_dispatch(&mut self, (program_id, message_id): (ProgramId, MessageId)) -> u64 {
        self.controller
            .update_state(program_id, |state, storage, _| {
                state.queue_hash.modify_queue(storage, |queue| {
                    let dispatch = state
                        .stash_hash
                        .modify_stash(storage, |stash| stash.remove_to_program(&message_id));

                    queue.queue(dispatch);
                });
            });

        0
    }

    fn send_user_message(&mut self, stashed_message_id: MessageId, program_id: ProgramId) -> u64 {
        self.controller
            .update_state(program_id, |state, storage, transitions| {
                let (dispatch, user_id) = state
                    .stash_hash
                    .modify_stash(storage, |stash| stash.remove_to_user(&stashed_message_id));

                let expiry = transitions.schedule_task(
                    MAILBOX_VALIDITY.try_into().expect("infallible"),
                    ScheduledTask::RemoveFromMailbox((program_id, user_id), stashed_message_id),
                );

                state.mailbox_hash.modify_mailbox(storage, |mailbox| {
                    mailbox.add_and_store_user_mailbox(
                        storage,
                        user_id,
                        stashed_message_id,
                        dispatch.clone().into(),
                        expiry,
                    );
                });

                transitions.modify_transition(program_id, |transition| {
                    transition
                        .messages
                        .push(dispatch.into_message(storage, user_id))
                })
            });

        0
    }

    // TODO (breathx): consider deprecation of delayed wakes + non-concrete waits.
    fn wake_message(&mut self, program_id: ProgramId, message_id: MessageId) -> u64 {
        log::trace!("Running scheduled task wake message {message_id} to {program_id}");

        self.controller
            .update_state(program_id, |state, storage, _| {
                let Expiring {
                    value: dispatch, ..
                } = state.waitlist_hash.modify_waitlist(storage, |waitlist| {
                    waitlist
                        .wake(&message_id)
                        .expect("failed to find message in waitlist")
                });

                state.queue_hash.modify_queue(storage, |queue| {
                    queue.queue(dispatch);
                })
            });

        0
    }

    /* Deprecated APIs */
    fn remove_from_waitlist(&mut self, _program_id: ProgramId, _message_id: MessageId) -> u64 {
        unreachable!("considering deprecation of it; use `wake_message` instead")
    }
    fn pause_program(&mut self, _: ProgramId) -> u64 {
        unreachable!("deprecated")
    }
    fn remove_code(&mut self, _: CodeId) -> u64 {
        unreachable!("deprecated")
    }
    fn remove_gas_reservation(&mut self, _: ProgramId, _: ReservationId) -> u64 {
        unreachable!("deprecated")
    }
    fn remove_paused_program(&mut self, _: ProgramId) -> u64 {
        unreachable!("deprecated")
    }
    fn remove_resume_session(&mut self, _: u32) -> u64 {
        unreachable!("deprecated")
    }
}
