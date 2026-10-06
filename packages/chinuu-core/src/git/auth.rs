use std::path::PathBuf;

use git2::{Config, Cred, CredentialType, RemoteCallbacks};

/// How to authenticate against a remote.
///
/// [`Auth::Default`] is the right choice when the user already has git set up:
/// it tries ssh-agent first, then the configured credential helper, which is
/// where a stored personal access token usually lives. The user's own git
/// configuration is therefore honoured rather than duplicated in app settings.
///
/// This type holds credentials, so its `Debug` is written by hand and redacts
/// the token and the passphrase. A derived implementation would print both in
/// full, which turns any log line, panic message or failing assertion into a
/// credential leak. It is also deliberately not serializable, for the same
/// reason.
#[derive(Clone, Default, PartialEq, Eq)]
pub enum Auth {
    /// ssh-agent, then the git credential helper, then whatever libgit2 finds.
    #[default]
    Default,
    /// A personal access token over https.
    Token { username: String, token: String },
    /// ssh-agent only, taking the username from the remote URL.
    SshAgent,
    /// A private key on disk, with an optional public key and passphrase.
    SshKey {
        username: String,
        private_key: PathBuf,
        public_key: Option<PathBuf>,
        passphrase: Option<String>,
    },
}

impl std::fmt::Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Auth::Default => f.write_str("Default"),
            Auth::SshAgent => f.write_str("SshAgent"),
            Auth::Token { username, .. } => f
                .debug_struct("Token")
                .field("username", username)
                .field("token", &"<redacted>")
                .finish(),
            Auth::SshKey {
                username,
                private_key,
                public_key,
                passphrase,
            } => f
                .debug_struct("SshKey")
                .field("username", username)
                .field("private_key", private_key)
                .field("public_key", public_key)
                // Whether a passphrase is set stays visible, because that is
                // what a caller debugging a failed login needs to know.
                .field("passphrase", &passphrase.as_ref().map(|_| "<redacted>"))
                .finish(),
        }
    }
}

impl Auth {
    /// Convenience constructor for a token.
    pub fn token(username: impl Into<String>, token: impl Into<String>) -> Self {
        Auth::Token {
            username: username.into(),
            token: token.into(),
        }
    }

    /// Convenience constructor for an unencrypted key with no separate public key.
    pub fn ssh_key(username: impl Into<String>, private_key: impl Into<PathBuf>) -> Self {
        Auth::SshKey {
            username: username.into(),
            private_key: private_key.into(),
            public_key: None,
            passphrase: None,
        }
    }
}

/// Build credential callbacks for one network operation.
///
/// The callback must be `'static`, so it takes ownership of the config handle.
/// `git2::Config` deliberately has no `Clone` impl, which is why this takes it
/// by value: borrowing it would tie the callbacks to the caller's stack frame,
/// and a `.clone()` here would silently clone the reference instead.
pub(crate) fn remote_callbacks(config: Config, auth: &Auth) -> RemoteCallbacks<'static> {
    let auth = auth.clone();

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, username_from_url, allowed| {
        match &auth {
            Auth::Token { username, token } => {
                if !allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
                    return Err(git2::Error::from_str(
                        "remote does not accept a username and token",
                    ));
                }
                return Cred::userpass_plaintext(username, token);
            }
            Auth::SshAgent => {
                if !allowed.contains(CredentialType::SSH_KEY) {
                    return Err(git2::Error::from_str("remote does not accept ssh keys"));
                }
                return Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"));
            }
            Auth::SshKey {
                username,
                private_key,
                public_key,
                passphrase,
            } => {
                if !allowed.contains(CredentialType::SSH_KEY) {
                    return Err(git2::Error::from_str("remote does not accept ssh keys"));
                }
                return Cred::ssh_key(
                    username,
                    public_key.as_deref(),
                    private_key,
                    passphrase.as_deref(),
                );
            }
            Auth::Default => {}
        }

        if allowed.contains(CredentialType::SSH_KEY) {
            if let Some(user) = username_from_url {
                if let Ok(credential) = Cred::ssh_key_from_agent(user) {
                    return Ok(credential);
                }
            }
        }

        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            if let Ok(credential) = Cred::credential_helper(&config, url, username_from_url) {
                return Ok(credential);
            }
        }

        Cred::default()
    });

    callbacks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_redacts_a_token() {
        let auth = Auth::token("ada", "ghp_supersecret_value");
        let rendered = format!("{auth:?}");

        assert!(
            !rendered.contains("ghp_supersecret_value"),
            "leaked: {rendered}"
        );
        // The parts needed to debug a failed login are still there.
        assert!(rendered.contains("ada"), "lost the username: {rendered}");
        assert!(rendered.contains("redacted"), "got {rendered}");
    }

    #[test]
    fn debug_output_redacts_a_passphrase() {
        let auth = Auth::SshKey {
            username: "git".to_owned(),
            private_key: PathBuf::from("/home/me/.ssh/id_ed25519"),
            public_key: None,
            passphrase: Some("hunter2".to_owned()),
        };
        let rendered = format!("{auth:?}");

        assert!(!rendered.contains("hunter2"), "leaked: {rendered}");
        assert!(
            rendered.contains("id_ed25519"),
            "lost the key path: {rendered}"
        );
        assert!(
            rendered.contains("Some"),
            "whether a passphrase is set is useful: {rendered}"
        );
    }

    #[test]
    fn debug_output_of_the_simple_variants_is_unchanged() {
        assert_eq!(format!("{:?}", Auth::Default), "Default");
        assert_eq!(format!("{:?}", Auth::SshAgent), "SshAgent");
    }
}
