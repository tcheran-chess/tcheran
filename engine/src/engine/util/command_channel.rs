// From icarus, authored by sp00ph

//! One-capacity spmc broadcast channel, allowing the UCI thread to send a message
//! to all search threads. The sending thread blocks until all receiving threads have
//! handled a sent message.

use std::{
    ptr,
    sync::{
        Arc,
        atomic::{
            AtomicPtr, AtomicU32,
            Ordering::{Acquire, Relaxed, Release},
        },
    },
};

struct Shared<M: Sync> {
    msg_ptr: AtomicPtr<M>,
    futex: AtomicU32,
    num_receivers: u32,
}

pub struct Sender<M: Sync> {
    shared: Arc<Shared<M>>,
}

pub struct Receiver<M: Sync> {
    shared: Arc<Shared<M>>,
    generation: bool,
}

/// Creates a channel with exactly `num_receivers` receivers.
///
/// The returned values are the singular sender, and an iterator yielding the `num_receivers`
/// receivers. Note that sending a message will block the sender until all receivers have handled
/// the message, so dropping any of the receivers will lead to a deadlock in the sender thread.
pub fn channel<M: Sync>(num_receivers: u32) -> (Sender<M>, impl Iterator<Item = Receiver<M>>) {
    let shared = Arc::new(Shared {
        msg_ptr: AtomicPtr::new(ptr::null_mut()),
        futex: AtomicU32::new(0),
        num_receivers,
    });

    let tx = Sender {
        shared: shared.clone(),
    };
    let rx_iter = std::iter::repeat_n(shared, num_receivers as usize).map(|shared| Receiver {
        shared,
        generation: true,
    });

    (tx, rx_iter)
}

fn pack_futex(threads: u32, generation: bool) -> u32 {
    debug_assert!(threads < (u32::MAX >> 1));
    threads | u32::from(generation) << 31
}

fn unpack_futex(futex: u32) -> (u32, bool) {
    let threads = futex & (u32::MAX >> 1);
    let generation = (futex >> 31) as u8;
    (threads, generation != 0)
}

impl<M: Sync> Sender<M> {
    /// Sends a message to all receivers. Will block until all receivers have handled the message.
    pub fn send(&mut self, msg: M) {
        let shared = &*self.shared;
        let (threads, generation) = unpack_futex(shared.futex.load(Relaxed));
        // Because any previous `send` call waited until all receivers handled the message, there should be no outstanding receivers.
        debug_assert_eq!(threads, 0);

        // SAFETY: Because the sender waits until all receivers handled the message, we can safely store a pointer to the local message
        // and have the receivers dereference that pointer.
        let msg_ref = &msg;
        let msg_ptr = ptr::from_ref(msg_ref).cast_mut();
        shared.msg_ptr.store(msg_ptr, Relaxed);

        let next_gen = !generation;
        // After writing the message, we update the generation and wake the receivers. We use Release here, and Acquire in the receivers,
        // to make sure that writing the message happens-before the receivers read it.
        shared
            .futex
            .store(pack_futex(shared.num_receivers, next_gen), Release);
        atomic_wait::wake_all(&raw const shared.futex);

        // Now we wait until the number of outstanding receivers reaches 0. The receivers all decrement using Release, and we load
        // using Acquire here, to ensure that any accesses of `msg` from receivers happen-before we return from `send`.
        let mut futex = shared.futex.load(Acquire);
        while unpack_futex(futex).0 != 0 {
            atomic_wait::wait(&shared.futex, futex);
            futex = shared.futex.load(Acquire);
        }

        // Sanity check to make any rogue readers trap immediately.
        shared.msg_ptr.store(ptr::null_mut(), Relaxed);
        // Sanity check to ensure that nothing invalidated `msg_ref`. Specifically, `msg` itself can't have been moved.
        let _ = msg_ref;
    }
}

impl<M: Sync> Receiver<M> {
    /// Waits for a message from the sending thread, calls `handler` on it and returns the result.
    pub fn recv<R, F: FnOnce(&M) -> R>(&mut self, handler: F) -> R {
        let shared = &*self.shared;

        // Wait until the message generation matches our local generation.
        let mut futex = shared.futex.load(Acquire);
        while unpack_futex(futex).1 != self.generation {
            atomic_wait::wait(&shared.futex, futex);
            futex = shared.futex.load(Acquire);
        }

        // We toggle the generation bit before calling the user defined handler. Otherwise, if the handler
        // unwinds and gets caught by `catch_unwind`, another `recv()` invocation would falsely believe there
        // is an available message, and dereference the (probably null) msg_ptr.
        self.generation = !self.generation;

        // SAFETY: The loop above, combined with the Release store in `send()` ensures that
        // the sender thread is done touching the message, so we can access it safely.
        let msg_ref = unsafe { &*shared.msg_ptr.load(Relaxed) };
        let ret = handler(msg_ref);

        // We've handled the message, so we can decrement the outstanding receiver count.
        // If we're the last thread to do so, we wake the sender thread.
        if unpack_futex(shared.futex.fetch_sub(1, Release)).0 == 1 {
            atomic_wait::wake_all(&raw const shared.futex);
        }

        ret
    }
}
