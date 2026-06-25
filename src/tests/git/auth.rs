use super::*;
use std::{fs, path::PathBuf};

#[test]
fn classifies_common_remote_url_shapes() {
    assert_eq!(classify_remote_url("https://github.com/asinglebit/guitar.git").protocol, AuthProtocol::Https);
    assert_eq!(classify_remote_url("http://example.com/repo.git").protocol, AuthProtocol::Http);
    assert_eq!(classify_remote_url("ssh://git@github.com/asinglebit/guitar.git").protocol, AuthProtocol::Ssh);
    assert_eq!(classify_remote_url("git@github.com:asinglebit/guitar.git").protocol, AuthProtocol::Ssh);
    assert_eq!(classify_remote_url("file:///tmp/repo.git").protocol, AuthProtocol::Local);
    assert_eq!(classify_remote_url("../repo.git").protocol, AuthProtocol::Local);
    assert_eq!(classify_remote_url("C:\\repo.git").protocol, AuthProtocol::Local);
    assert_eq!(classify_remote_url("git://github.com/asinglebit/guitar.git").protocol, AuthProtocol::Other);
}

#[test]
fn extracts_username_and_host_from_remote_urls() {
    let ssh = classify_remote_url("git@github.com:asinglebit/guitar.git");
    assert_eq!(ssh.username.as_deref(), Some("git"));
    assert_eq!(ssh.host.as_deref(), Some("github.com"));

    let https = classify_remote_url("https://token@github.com/asinglebit/guitar.git");
    assert_eq!(https.username.as_deref(), Some("token"));
    assert_eq!(https.host.as_deref(), Some("github.com"));
}

#[test]
fn session_stores_and_evicts_https_secret() {
    let challenge = AuthChallenge { url: "https://github.com/asinglebit/guitar.git".to_string(), username: None, protocol: AuthProtocol::Https, operation: "Fetch".to_string(), key_path: None };
    let mut session = AuthSession::default();
    session.store(&challenge, AuthSecret::Https { username: "user".to_string(), password: "token".to_string() });

    let (key, username, password) = session.https_secret(&challenge.url, None, AuthProtocol::Https).unwrap();
    assert_eq!(username, "user");
    assert_eq!(password, "token");

    session.evict(&[key]);
    assert!(session.https_secret(&challenge.url, None, AuthProtocol::Https).is_none());
}

#[test]
fn session_stores_and_evicts_ssh_passphrase() {
    let challenge = AuthChallenge {
        url: "ssh://git@github.com/asinglebit/guitar.git".to_string(),
        username: Some("git".to_string()),
        protocol: AuthProtocol::Ssh,
        operation: "Fetch".to_string(),
        key_path: Some(PathBuf::from("/tmp/id_ed25519")),
    };
    let mut session = AuthSession::default();
    session.store(&challenge, AuthSecret::SshKeyPassphrase { passphrase: "secret".to_string() });

    let (key, passphrase) = session.ssh_secret(&challenge.url, "git", challenge.key_path.as_deref().unwrap()).unwrap();
    assert_eq!(passphrase, "secret");

    session.evict(&[key]);
    assert!(session.ssh_secret(&challenge.url, "git", challenge.key_path.as_deref().unwrap()).is_none());
}

#[test]
fn default_ssh_private_key_prefers_ed25519_then_ecdsa_then_rsa() {
    let home = tempfile::Builder::new().prefix("guitar-auth-ssh-key-order-").tempdir().unwrap();
    let home = home.path();
    let ssh = home.join(".ssh");
    fs::create_dir_all(&ssh).unwrap();

    let rsa = ssh.join("id_rsa");
    let ecdsa = ssh.join("id_ecdsa");
    let ed25519 = ssh.join("id_ed25519");

    fs::write(&rsa, "rsa").unwrap();
    fs::write(&ecdsa, "ecdsa").unwrap();
    fs::write(&ed25519, "ed25519").unwrap();

    assert_eq!(default_ssh_private_key_in(&home).as_deref(), Some(ed25519.as_path()));

    fs::remove_file(&ed25519).unwrap();
    assert_eq!(default_ssh_private_key_in(&home).as_deref(), Some(ecdsa.as_path()));

    fs::remove_file(&ecdsa).unwrap();
    assert_eq!(default_ssh_private_key_in(&home).as_deref(), Some(rsa.as_path()));
}

#[test]
fn auth_required_result_carries_challenge_and_rejected_session_keys() {
    let attempt = AuthAttempt::new(AuthSession::default(), "Fetch");
    let challenge = AuthChallenge {
        url: "https://github.com/asinglebit/guitar.git".to_string(),
        username: Some("user".to_string()),
        protocol: AuthProtocol::Https,
        operation: "Fetch".to_string(),
        key_path: None,
    };
    let rejected = AuthCacheKey::https(&challenge.url, "user", AuthProtocol::Https);

    attempt.set_challenge(challenge.clone());
    attempt.record_used(rejected.clone());

    match network_result("Fetch", &attempt, Err(auth_required_error())) {
        NetworkResult::AuthRequired(auth) => {
            assert_eq!(auth.challenge, challenge);
            assert_eq!(auth.rejected, vec![rejected]);
        },
        other => panic!("expected auth challenge, got {other:?}"),
    }
}
