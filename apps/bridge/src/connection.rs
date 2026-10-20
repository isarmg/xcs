//! Login attempts, retained credentials, and in-flight work have separate
//! identities. Logout fences old work before revocation finishes, while retaining
//! the credential for retry; stale completions cannot replace a newer session.
use zxs_client::Client;

#[derive(Default)]
pub(super) struct Connection {
    client: Option<Client>,
    revision: u64,
    attempt: u64,
    work_epoch: u64,
}
impl Connection {
    pub(super) fn begin(&mut self) -> u64 {
        self.attempt = self.attempt.wrapping_add(1);
        self.attempt
    }
    pub(super) fn install(&mut self, attempt: u64, client: Client) -> bool {
        if self.attempt != attempt {
            return false;
        }
        self.revision = self.revision.wrapping_add(1);
        self.work_epoch = self.work_epoch.wrapping_add(1);
        self.client = Some(client);
        true
    }
    pub(super) fn snapshot(&self) -> Option<(Client, u64)> {
        self.client.clone().map(|client| (client, self.revision))
    }
    pub(super) fn work_snapshot(&self) -> Option<(Client, u64)> {
        self.client.clone().map(|client| (client, self.work_epoch))
    }
    pub(super) fn work_current(&self, epoch: u64) -> bool {
        self.client.is_some() && self.work_epoch == epoch
    }
    /// Fence already-started work immediately, without losing the bearer needed
    /// to retry an unconfirmed logout. New explicit work may use the new epoch.
    pub(super) fn begin_logout(&mut self) -> Option<(Client, u64)> {
        self.begin();
        self.work_epoch = self.work_epoch.wrapping_add(1);
        self.snapshot()
    }
    pub(super) fn current(&self, revision: u64) -> bool {
        self.client.is_some() && self.revision == revision
    }
    pub(super) fn clear(&mut self) {
        self.begin();
        self.revision = self.revision.wrapping_add(1);
        self.work_epoch = self.work_epoch.wrapping_add(1);
        self.client = None;
    }
}
