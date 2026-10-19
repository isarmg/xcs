//! Connection attempts and active sessions have separate identities: a failed
//! login keeps the old connection, and stale completions cannot replace it.
use xcs_client::Client;

#[derive(Default)]
pub(super) struct Connection {
    client: Option<Client>,
    revision: u64,
    attempt: u64,
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
        self.client = Some(client);
        true
    }
    pub(super) fn snapshot(&self) -> Option<(Client, u64)> {
        self.client.clone().map(|client| (client, self.revision))
    }
    pub(super) fn current(&self, revision: u64) -> bool {
        self.client.is_some() && self.revision == revision
    }
    pub(super) fn clear(&mut self) {
        self.begin();
        self.revision = self.revision.wrapping_add(1);
        self.client = None;
    }
    pub(super) fn clear_if(&mut self, revision: u64) {
        if self.current(revision) {
            self.clear();
        }
    }
}
