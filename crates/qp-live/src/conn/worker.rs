//! The connection worker thread: its run loop, back-off, halts and the subscription
//! commands (split out of `conn.rs` for size).

use super::*;

impl Worker {
    pub(super) fn event(&self, e: LiveEvent) {
        self.sink.on_event(&self.key, &e);
    }

    pub(super) fn mask(&self, s: &str) -> String {
        self.redactor.scrub(s)
    }

    pub(super) fn run(mut self) {
        for topic in std::mem::take(&mut self.refused_at_start) {
            self.refuse_level2(topic);
        }
        let mut backoff = Backoff::new(self.opts.backoff.clone());
        let mut renewed: Option<AccessToken> = None;
        loop {
            if self.drain_commands() {
                break;
            }
            let token = if let Some(t) = renewed.take() {
                Ok(t)
            } else if self.force_refresh {
                self.auth.refresh()
            } else {
                self.auth.token()
            };
            let token = match token {
                Ok(t) => {
                    self.force_refresh = false;
                    t
                }
                Err(AuthError::RateLimited { wait }) => {
                    if self.sleep(wait) {
                        break;
                    }
                    continue;
                }
                Err(AuthError::Halted { .. }) => return self.halt(HaltReason::AuthUnavailable),
                Err(error) => {
                    self.event(LiveEvent::AuthFailed { error });
                    if self.back_off(&mut backoff) {
                        return;
                    }
                    continue;
                }
            };
            let attempt = backoff.failures() + 1;
            self.shared.set(ConnState::Connecting { attempt });
            self.event(LiveEvent::Connecting { attempt });
            match self.session(&token) {
                SessionEnd::Stopped => break,
                SessionEnd::Halt(reason) => return self.halt(reason),
                SessionEnd::AuthRefused(code) => {
                    self.event(LiveEvent::Disconnected {
                        reason: DisconnectReason::ConnackRefused(code),
                    });
                    if self.auth_retry_used {
                        return self.halt(HaltReason::AuthRejected { code });
                    }
                    self.auth_retry_used = true;
                    self.force_refresh = true;
                    self.event(LiveEvent::TokenRefreshForced { code });
                }
                SessionEnd::Renew(t) => {
                    renewed = Some(t);
                    self.event(LiveEvent::Disconnected {
                        reason: DisconnectReason::TokenRenewal,
                    });
                }
                SessionEnd::Lost { reason, stable } => {
                    let unexplained = is_unexplained_server_close(&reason);
                    self.event(LiveEvent::Disconnected { reason });
                    if unexplained {
                        if let Some(halt) = self.server_closes.record((self.opts.clock)()) {
                            return self.halt(halt);
                        }
                    }
                    if stable {
                        backoff.reset();
                    }
                    if self.back_off(&mut backoff) {
                        return;
                    }
                }
            }
        }
        self.finish_closed();
    }

    /// Releases the locks, then publishes `Closed`.
    fn finish_closed(&mut self) {
        self.lock = None;
        self.shared.set(ConnState::Closed);
        self.event(LiveEvent::Closed);
    }

    /// Records a failure and sleeps. Returns true if the worker must end (halted or stopped).
    fn back_off(&mut self, backoff: &mut Backoff) -> bool {
        match backoff.record_failure() {
            BackoffStep::Retry { failures, delay } => {
                self.shared.set(ConnState::BackingOff { failures });
                self.event(LiveEvent::BackingOff { failures, delay });
                if self.sleep(delay) {
                    self.finish_closed();
                    return true;
                }
                false
            }
            BackoffStep::Stop { failures } => {
                self.halt(HaltReason::TooManyFailures { failures });
                true
            }
        }
    }

    /// Bans (multi-connect, unexplained server closes), releases the locks, then publishes
    /// the final `Halted` state.
    fn halt(&mut self, reason: HaltReason) {
        if reason.needs_person_reset() {
            mark_halted(&self.opts.lock_dir, &self.key, &reason);
        }
        self.lock = None;
        self.shared.set(ConnState::Halted(reason.clone()));
        self.event(LiveEvent::Halted { reason });
    }

    pub(super) fn add_topics(&mut self, topics: Vec<String>) -> Vec<String> {
        let mut added = Vec::new();
        for t in topics {
            if is_level2_trade_topic(&t) {
                self.refuse_level2(t);
            } else if !t.is_empty() && self.topics.insert(t.clone()) {
                added.push(t);
            }
        }
        added
    }

    /// A level-2 trade topic is refused here; nothing goes to the server.
    fn refuse_level2(&self, topic: String) {
        self.event(LiveEvent::SubscribeRejected {
            topic: self.mask(&topic),
            code: SUBACK_FAILURE,
        });
    }

    pub(super) fn remove_topics(&mut self, topics: Vec<String>) -> Vec<String> {
        topics
            .into_iter()
            .filter(|t| self.topics.remove(t))
            .collect()
    }

    /// Applies pending commands; true if stop was requested.
    fn drain_commands(&mut self) -> bool {
        loop {
            match self.rx.try_recv() {
                Ok(Command::Stop) | Err(TryRecvError::Disconnected) => return true,
                Ok(Command::Subscribe(t)) => {
                    self.add_topics(t);
                }
                Ok(Command::Unsubscribe(t)) => {
                    self.remove_topics(t);
                }
                Err(TryRecvError::Empty) => return false,
            }
        }
    }

    /// Sleeps while applying commands; true if stop was requested.
    fn sleep(&mut self, d: Duration) -> bool {
        let deadline = Instant::now() + d;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            match self.rx.recv_timeout(deadline - now) {
                Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => return true,
                Ok(Command::Subscribe(t)) => {
                    self.add_topics(t);
                }
                Ok(Command::Unsubscribe(t)) => {
                    self.remove_topics(t);
                }
                Err(RecvTimeoutError::Timeout) => return false,
            }
        }
    }

    pub(super) fn batches(&self, topics: impl IntoIterator<Item = String>) -> Vec<Vec<String>> {
        let n = self.opts.subscribe_batch.max(1);
        let all: Vec<String> = topics.into_iter().collect();
        all.chunks(n).map(|c| c.to_vec()).collect()
    }
}
