//! Finds the user's logins for the registries a build downloads from, so a task can start from a private base image.
//!
//! They're the logins Docker has from `docker login`, found the way `docker pull` finds them: `DOCKER_AUTH_CONFIG`
//! first, then the credential helper that Docker's config names for the registry, or else the logins written in the
//! config itself. Only the registries a build uses are looked up, so a build never gets the user's other logins and a
//! credential helper is only asked about those.
//!
//! <https://github.com/docker/cli/blob/v29.10.1/cli/config/configfile/file.go#L318-L402>
//!
//! BuildKit runs in a container, where it can't see any of that, so [`Logins::docker_config`] writes what was found into
//! a Docker config of its own, which goes in with the build's files.

use base64::{Engine, engine::general_purpose::STANDARD};
use oci_client::secrets::RegistryAuth;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    io,
    path::PathBuf,
    process::Stdio,
};
use tokio::io::AsyncWriteExt;

/// What Docker's config calls Docker Hub, whichever of Hub's names an image uses.
///
/// <https://github.com/docker/cli/blob/v29.10.1/cli/config/configfile/file.go#L44-L49>
const DOCKER_HUB_KEY: &str = "https://index.docker.io/v1/";

/// What a credential helper says when it has no login for a registry.
///
/// <https://github.com/docker/docker-credential-helpers/blob/v0.9.13/credentials/error.go#L11>
const HELPER_NOT_FOUND: &str = "credentials not found in native keychain";

/// The parts of Docker's config that say where its logins are.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(default)]
    auths: HashMap<String, Login>,

    /// The credential helper for every registry that `cred_helpers` doesn't name.
    #[serde(default)]
    creds_store: String,

    #[serde(default)]
    cred_helpers: HashMap<String, String>,
}

/// A login, in the fields Docker's config has for one. It has no `Debug`, so it can't end up in a log or an error.
#[derive(Deserialize, Default, Clone)]
struct Login {
    /// The username and password, as base64 of `username:password`. Docker uses this over the two fields below.
    #[serde(default)]
    auth: String,

    #[serde(default)]
    username: String,

    #[serde(default)]
    password: String,

    /// A token that some registries hand out in place of a password, which is traded for a registry token.
    #[serde(default)]
    identitytoken: String,

    /// A token that's sent to the registry as it is.
    #[serde(default)]
    registrytoken: String,
}

impl Login {
    /// Docker reads `auth` into the username and password when it loads its config, so this does the same. `None` when
    /// it isn't one Docker could read.
    ///
    /// <https://github.com/docker/cli/blob/v29.10.1/cli/config/configfile/file.go#L295-L316>
    fn decoded(mut self) -> Option<Self> {
        if self.auth.is_empty() {
            return Some(self);
        }

        let decoded = String::from_utf8(STANDARD.decode(self.auth.trim()).ok()?).ok()?;
        let (username, password) = decoded.split_once(':')?;
        if username.is_empty() {
            return None;
        }

        self.username = username.into();
        self.password = password.trim_matches('\0').into();
        Some(self)
    }

    fn is_empty(&self) -> bool {
        self.username.is_empty() && self.identitytoken.is_empty() && self.registrytoken.is_empty()
    }
}

/// What came of looking up the login for one registry.
enum Found {
    /// Docker has none, so downloads from it are anonymous.
    Nothing,

    Login(Login),

    /// Docker says where its login is, but it couldn't be read. This says why, and never has the login in it.
    Failed(String),
}

/// The user's logins for the registries that one build downloads from, by registry.
#[derive(Default)]
pub struct Logins(BTreeMap<String, Found>);

/// Looks up the logins for the registries these images are in.
pub async fn find(images: &[&str]) -> Logins {
    let mut found = BTreeMap::new();
    let mut sources = None;

    for image in images {
        if image.eq_ignore_ascii_case("scratch") {
            continue;
        }

        let registry = registry(image);
        if found.contains_key(registry) {
            continue;
        }

        let (env, config) = sources.get_or_insert_with(|| (env_logins(), read_config()));
        found.insert(registry.to_string(), look_up(env, config, registry).await);
    }

    Logins(found)
}

impl Logins {
    /// A Docker config with the logins that were found, for BuildKit.
    pub fn docker_config(&self) -> Vec<u8> {
        let mut auths = serde_json::Map::new();

        for (registry, found) in &self.0 {
            let Found::Login(login) = found else {
                continue;
            };

            let mut entry = serde_json::Map::new();
            // Docker can't read an `auth` without a username, and then it ignores the whole config.
            if !login.username.is_empty() {
                let auth = format!("{}:{}", login.username, login.password);
                entry.insert("auth".into(), STANDARD.encode(auth).into());
            }
            if !login.identitytoken.is_empty() {
                entry.insert("identitytoken".into(), login.identitytoken.clone().into());
            }
            if !login.registrytoken.is_empty() {
                entry.insert("registrytoken".into(), login.registrytoken.clone().into());
            }

            auths.insert(config_key(registry).into(), entry.into());
        }

        serde_json::json!({ "auths": auths })
            .to_string()
            .into_bytes()
    }

    /// For downloading without BuildKit. oci-client can't use an identity token: it has to be traded for a registry
    /// token, and oci-client only knows how to trade a username and password. So a login that's an identity token
    /// counts as one that couldn't be used, and the error says so if the registry wants a login.
    pub fn without_identity_tokens(mut self) -> Self {
        for found in self.0.values_mut() {
            if let Found::Login(login) = found
                && !login.identitytoken.is_empty()
                && login.registrytoken.is_empty()
            {
                *found = Found::Failed(
                    "it's an identity token, which Gofer can only use when it builds the image with Docker".into(),
                );
            }
        }

        self
    }

    /// The login for the registry an image is in, for oci-client.
    pub fn registry_auth(&self, image: &str) -> RegistryAuth {
        match self.0.get(registry(image)) {
            Some(Found::Login(login)) if !login.registrytoken.is_empty() => {
                RegistryAuth::Bearer(login.registrytoken.clone())
            }
            Some(Found::Login(login)) if !login.username.is_empty() => {
                RegistryAuth::Basic(login.username.clone(), login.password.clone())
            }
            _ => RegistryAuth::Anonymous,
        }
    }

    /// What to tell the user when an image's registry wouldn't let it be downloaded. Docker Hub says the same about
    /// images that don't exist, so it could be either.
    pub fn needs_login(&self, image: &str) -> String {
        let registry = registry(image);
        let name = registry_name(image);

        match self.0.get(registry) {
            Some(Found::Login(_)) => format!(
                "The base image {image} doesn't exist, or your Docker login for {name} isn't allowed to download it."
            ),
            Some(Found::Failed(why)) => format!(
                "The base image {image} doesn't exist, or it needs a login to download and your Docker login for \
                 {name} couldn't be used; {}.",
                why.trim_end_matches('.')
            ),
            Some(Found::Nothing) | None => {
                let command = match registry {
                    "docker.io" => "docker login".to_string(),
                    _ => format!("docker login {registry}"),
                };
                format!(
                    "The base image {image} doesn't exist, or it needs a login to download. To log in, run \
                     `{command}`; builds use the same logins Docker does."
                )
            }
        }
    }
}

/// The registry an image is in, like `ghcr.io` for `ghcr.io/org/app:1`. Images without one are on Docker Hub, which is
/// `docker.io` however the image names it.
///
/// <https://github.com/distribution/reference/blob/v0.6.0/normalize.go#L126-L160>
pub fn registry(image: &str) -> &str {
    match image.split_once('/') {
        Some(("index.docker.io" | "registry-1.docker.io", _)) => "docker.io",
        Some((host, _)) if host.contains(['.', ':']) || host == "localhost" => host,
        _ => "docker.io",
    }
}

/// The registry an image is in, the way people say it.
pub fn registry_name(image: &str) -> &str {
    match registry(image) {
        "docker.io" => "Docker Hub",
        registry => registry,
    }
}

/// The name Docker's config gives a registry.
fn config_key(registry: &str) -> &str {
    match registry {
        "docker.io" => DOCKER_HUB_KEY,
        registry => registry,
    }
}

/// Logins from `DOCKER_AUTH_CONFIG`, which CI systems set in place of writing a config. Docker carries on without it
/// when it can't be read, and so do we.
fn env_logins() -> HashMap<String, Login> {
    #[derive(Deserialize)]
    struct Env {
        #[serde(default)]
        auths: HashMap<String, Login>,
    }

    (std::env::var("DOCKER_AUTH_CONFIG").ok())
        .and_then(|env| serde_json::from_str::<Env>(&env).ok())
        .map(|env| env.auths)
        .unwrap_or_default()
}

/// Reads Docker's config, from the same place Docker does. Without one, there are no logins. Docker carries on without
/// a config it can't read, and so do we, but this says why so a login that's missing because of it can be explained.
fn read_config() -> Result<Config, String> {
    let folder = match std::env::var_os("DOCKER_CONFIG") {
        Some(folder) => PathBuf::from(folder),
        None => match dirs::home_dir() {
            Some(home) => home.join(".docker"),
            None => return Ok(Config::default()),
        },
    };
    let path = folder.join("config.json");

    let content = match std::fs::read(&path) {
        Ok(content) => content,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(e) => {
            return Err(format!(
                "Docker's config at {} can't be read; {e}",
                path.display()
            ));
        }
    };

    // serde's message can quote what it didn't expect, which could be a password, so this only says where.
    serde_json::from_slice(&content).map_err(|e| {
        format!(
            "Docker's config at {} isn't valid JSON, at line {} column {}",
            path.display(),
            e.line(),
            e.column()
        )
    })
}

/// Finds the login for one registry the way Docker does.
async fn look_up(
    env: &HashMap<String, Login>,
    config: &Result<Config, String>,
    registry: &str,
) -> Found {
    let key = config_key(registry);

    if let Some(login) = env.get(key) {
        return found(login.clone());
    }

    let config = match config {
        Ok(config) => config,
        Err(why) => return Found::Failed(why.clone()),
    };

    // A helper named for the registry wins over the default one, even when it's empty, which means not to use a helper.
    let helper = config.cred_helpers.get(key).unwrap_or(&config.creds_store);
    if !helper.is_empty() {
        return from_helper(&format!("docker-credential-{helper}"), key).await;
    }

    // The config can name a registry by a URL, like `https://ghcr.io/v2/`, which means the same as `ghcr.io`.
    let login = (config.auths.get(key)).or_else(|| {
        (config.auths.iter())
            .find(|(name, _)| hostname(name) == key)
            .map(|(_, login)| login)
    });

    match login {
        Some(login) => found(login.clone()),
        None => Found::Nothing,
    }
}

fn found(login: Login) -> Found {
    match login.decoded() {
        // Docker leaves empty entries behind, like after `docker logout`.
        Some(login) if login.is_empty() => Found::Nothing,
        Some(login) => Found::Login(login),
        None => Found::Failed(
            "its entry in Docker's config isn't a username and password Docker can read".into(),
        ),
    }
}

/// `https://ghcr.io/v2/` becomes `ghcr.io`.
///
/// <https://github.com/docker/cli/blob/v29.10.1/cli/config/credentials/file_store.go#L110-L127>
fn hostname(name: &str) -> &str {
    let name = name.split_once("://").map_or(name, |(_, rest)| rest);
    name.split('/').next().unwrap_or(name)
}

/// Asks a credential helper for a registry's login. A helper is a program called `docker-credential-<name>`, whose
/// `get` reads the registry from stdin and answers with JSON.
///
/// <https://github.com/docker/docker-credential-helpers/blob/v0.9.13/client/client.go#L46-L74>
async fn from_helper(program: &str, key: &str) -> Found {
    let spawned = tokio::process::Command::new(program)
        .arg("get")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Found::Failed(format!(
                "Docker's config says to get it from {program}, which isn't installed"
            ));
        }
        Err(e) => return Found::Failed(format!("{program} didn't start; {e}")),
    };

    // Dropping stdin closes it, which is how the helper knows the registry's name is all there.
    if let Some(mut stdin) = child.stdin.take() {
        _ = stdin.write_all(key.as_bytes()).await;
    }

    let output = match child.wait_with_output().await {
        Ok(output) => output,
        Err(e) => return Found::Failed(format!("{program} failed; {e}")),
    };

    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stdout);
        if said.trim() == HELPER_NOT_FOUND {
            return Found::Nothing;
        }

        let errors = String::from_utf8_lossy(&output.stderr);
        let why = [said.trim(), errors.trim()]
            .into_iter()
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("; ");
        return Found::Failed(match why.is_empty() {
            true => format!("{program} failed with {}", output.status),
            false => format!("{program} failed: {why}"),
        });
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Answer {
        #[serde(default)]
        username: String,

        #[serde(default)]
        secret: String,
    }

    let Ok(answer) = serde_json::from_slice::<Answer>(&output.stdout) else {
        return Found::Failed(format!("{program} didn't answer with a login"));
    };

    // Helpers keep an identity token as a login with this for its username.
    //
    // <https://github.com/docker/cli/blob/v29.10.1/cli/config/credentials/native_store.go#L9-L12>
    found(match answer.username.as_str() {
        "<token>" => Login {
            identitytoken: answer.secret,
            ..Default::default()
        },
        _ => Login {
            username: answer.username,
            password: answer.secret,
            ..Default::default()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use std::os::unix::fs::PermissionsExt;

    fn config(json: serde_json::Value) -> Result<Config, String> {
        Ok(serde_json::from_value(json).unwrap())
    }

    fn auth(username: &str, password: &str) -> String {
        STANDARD.encode(format!("{username}:{password}"))
    }

    /// The username and password that were found, or what came of it.
    fn describe(found: &Found) -> String {
        match found {
            Found::Nothing => "nothing".into(),
            Found::Login(login) if !login.identitytoken.is_empty() => {
                format!("identity token {}", login.identitytoken)
            }
            Found::Login(login) => format!("{}:{}", login.username, login.password),
            Found::Failed(why) => format!("failed: {why}"),
        }
    }

    #[rstest]
    #[case("alpine", "docker.io")]
    #[case("alpine:3", "docker.io")]
    #[case("someone/private:1", "docker.io")]
    #[case("docker.io/library/alpine:3", "docker.io")]
    #[case("index.docker.io/someone/private", "docker.io")]
    #[case("ghcr.io/org/app:1", "ghcr.io")]
    #[case("localhost/app", "localhost")]
    #[case("localhost:5000/app", "localhost:5000")]
    #[case("registry:5000/app@sha256:abc", "registry:5000")]
    fn test_registries(#[case] image: &str, #[case] expected: &str) {
        assert_eq!(registry(image), expected);
    }

    #[tokio::test]
    async fn test_logins_come_from_the_config() {
        let config = config(serde_json::json!({
            "auths": {
                "ghcr.io": {"auth": auth("me", "pass:word")},
                "https://quay.io/v2/": {"auth": auth("quay", "secret")},
                "https://index.docker.io/v1/": {"auth": auth("hub", "token")},
                "registry.example.com": {"username": "plain", "password": "text"},
                "acr.example.com": {"auth": auth("00000000", ""), "identitytoken": "refresh"},
                "logged-out.example.com": {},
                "broken.example.com": {"auth": "not base64!"},
            },
        }));
        let none = HashMap::new();

        for (registry, expected) in [
            // Passwords can have colons in them.
            ("ghcr.io", "me:pass:word"),
            ("quay.io", "quay:secret"),
            ("docker.io", "hub:token"),
            ("registry.example.com", "plain:text"),
            ("acr.example.com", "identity token refresh"),
            ("logged-out.example.com", "nothing"),
            ("elsewhere.example.com", "nothing"),
        ] {
            let found = look_up(&none, &config, registry).await;
            assert_eq!(describe(&found), expected, "{registry}");
        }

        let found = look_up(&none, &config, "broken.example.com").await;
        assert!(describe(&found).starts_with("failed: its entry"));
    }

    #[tokio::test]
    async fn test_env_comes_first_and_a_broken_config_is_explained() {
        let env = HashMap::from([(
            DOCKER_HUB_KEY.to_string(),
            Login {
                auth: auth("ci", "token"),
                ..Default::default()
            },
        )]);
        let broken: Result<Config, String> = Err("Docker's config isn't valid JSON".into());

        assert_eq!(
            describe(&look_up(&env, &broken, "docker.io").await),
            "ci:token"
        );
        assert_eq!(
            describe(&look_up(&env, &broken, "ghcr.io").await),
            "failed: Docker's config isn't valid JSON"
        );
    }

    #[tokio::test]
    async fn test_helpers_come_before_the_config() {
        let config = config(serde_json::json!({
            "auths": {"ghcr.io": {"auth": auth("file", "login")}, "quay.io": {"auth": auth("file", "login")}},
            "credsStore": "gofer-test-not-installed",
            "credHelpers": {"quay.io": ""},
        }));
        let none = HashMap::new();

        let found = describe(&look_up(&none, &config, "ghcr.io").await);
        assert_eq!(
            found,
            "failed: Docker's config says to get it from docker-credential-gofer-test-not-installed, which isn't \
             installed"
        );

        // An empty helper means to use the config.
        assert_eq!(
            describe(&look_up(&none, &config, "quay.io").await),
            "file:login"
        );
    }

    #[tokio::test]
    async fn test_helpers_are_asked_like_docker_does() {
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("docker-credential-test");
        std::fs::write(
            &helper,
            r#"#!/bin/sh
[ "$1" = get ] || exit 2
registry=$(cat)
case "$registry" in
    https://index.docker.io/v1/) echo '{"ServerURL":"hub","Username":"me","Secret":"pass"}' ;;
    token.example.com) echo '{"ServerURL":"x","Username":"<token>","Secret":"refresh"}' ;;
    broken.example.com) echo "the keyring is locked" >&2; exit 1 ;;
    *) echo "credentials not found in native keychain"; exit 1 ;;
esac
"#,
        )
        .unwrap();
        std::fs::set_permissions(&helper, PermissionsExt::from_mode(0o755)).unwrap();
        let helper = helper.to_str().unwrap();

        for (key, expected) in [
            (DOCKER_HUB_KEY, "me:pass".to_string()),
            ("token.example.com", "identity token refresh".into()),
            ("ghcr.io", "nothing".into()),
            (
                "broken.example.com",
                format!("failed: {helper} failed: the keyring is locked"),
            ),
        ] {
            assert_eq!(describe(&from_helper(helper, key).await), expected, "{key}");
        }
    }

    fn logins() -> Logins {
        let login = |username: &str, identitytoken: &str| {
            Found::Login(Login {
                username: username.into(),
                password: "pass".into(),
                identitytoken: identitytoken.into(),
                ..Default::default()
            })
        };

        Logins(BTreeMap::from([
            ("docker.io".to_string(), login("me", "")),
            ("acr.example.com".to_string(), login("00000000", "refresh")),
            ("ghcr.io".to_string(), Found::Nothing),
            (
                "quay.io".to_string(),
                Found::Failed("docker-credential-pass failed: locked.".into()),
            ),
        ]))
    }

    #[test]
    fn test_buildkit_gets_only_what_was_found() {
        let config: serde_json::Value = serde_json::from_slice(&logins().docker_config()).unwrap();

        assert_eq!(
            config,
            serde_json::json!({"auths": {
                "https://index.docker.io/v1/": {"auth": auth("me", "pass")},
                "acr.example.com": {"auth": auth("00000000", "pass"), "identitytoken": "refresh"},
            }})
        );
    }

    #[test]
    fn test_downloads_without_buildkit_skip_identity_tokens() {
        let logins = logins().without_identity_tokens();

        assert_eq!(
            logins.registry_auth("alpine:3"),
            RegistryAuth::Basic("me".into(), "pass".into())
        );
        assert_eq!(
            logins.registry_auth("acr.example.com/app"),
            RegistryAuth::Anonymous
        );
        assert_eq!(
            logins.registry_auth("ghcr.io/org/app"),
            RegistryAuth::Anonymous
        );
        assert!(logins.needs_login("acr.example.com/app").ends_with(
            "it's an identity token, which Gofer can only use when it builds the image with Docker."
        ));
    }

    #[rstest]
    #[case(
        "someone/private:1",
        "The base image someone/private:1 doesn't exist, or your Docker login for Docker Hub isn't allowed to download it."
    )]
    #[case(
        "ghcr.io/org/app:1",
        "The base image ghcr.io/org/app:1 doesn't exist, or it needs a login to download. To log in, run `docker login \
         ghcr.io`; builds use the same logins Docker does."
    )]
    #[case(
        "registry.example.com/app",
        "The base image registry.example.com/app doesn't exist, or it needs a login to download. To log in, run \
         `docker login registry.example.com`; builds use the same logins Docker does."
    )]
    #[case(
        "quay.io/org/app",
        "The base image quay.io/org/app doesn't exist, or it needs a login to download and your Docker login for \
         quay.io couldn't be used; docker-credential-pass failed: locked."
    )]
    fn test_needing_a_login_says_what_to_do(#[case] image: &str, #[case] expected: &str) {
        assert_eq!(logins().needs_login(image), expected);
    }
}
