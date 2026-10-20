//! Shared presentation components. Authentication and business I/O are injected by the application.
mod account;
mod dialog;
mod header;
mod login;
mod notification;
mod select;
mod shell;
mod state;
pub use account::AccountPage;
pub use dialog::*;
pub use header::{HeaderActions, HeaderNavigation, LanguageToggle, ThemeToggle};
pub use login::LoginPage;
pub use notification::notify;
pub use select::{Select, SelectOption};
pub use shell::AppShell;
pub use state::*;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Feedback {
    pub message: String,
    pub request_id: Option<String>,
}
impl From<&str> for Feedback {
    fn from(message: &str) -> Self {
        Self {
            message: message.into(),
            request_id: None,
        }
    }
}
impl Feedback {
    pub fn display(&self) -> String {
        match &self.request_id {
            Some(id) if safe_request_id(id) => format!("{} · {}", self.message, id),
            _ => self.message.clone(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SessionPhase {
    Loading,
    #[default]
    Anonymous,
    Authenticated,
    Error,
    LogoutUnconfirmed,
}
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SessionView {
    pub phase: SessionPhase,
    pub username: String,
    pub admin: bool,
    pub message: String,
    pub request_id: Option<String>,
}
// These values deliberately do not implement Debug: they contain credentials.
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
}
pub struct PasswordChange {
    pub current_password: String,
    pub new_password: String,
}
pub(crate) fn safe_request_id(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}
