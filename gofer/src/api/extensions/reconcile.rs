//! Keeps the extensions Gofer runs in line with the `[[extensions.install]]` entries in Gofer's config.
//!
//! Gofer's config is the only place extensions are installed or changed. When Gofer starts it brings every extension
//! in line with the config on its own. While it's running, `gofer extension reload` does the same thing in steps the
//! operator can follow: it asks for a plan (what would change and why), then applies each extension one at a time,
//! and can revert one whose new version won't start.
//!
//! Applying an extension always means stopping the old container before starting the new one. Running both at once
//! would have two copies acting on the same pipeline subscriptions.

use super::{
    Documentation, Extension, ExtensionPathArgs, Parameter, Registration, State, Status,
    install_new_extension, start_extension, stop_extension,
};
use crate::{
    api::{
        ApiState, InterpolationKind, PreflightOptions, RegistryAuth, Variable, VariableSource,
        epoch_milli, fetch_global_secret, is_valid_identifier, parse_interpolation_syntax,
        permissioning::{Action, Requirement},
    },
    conf::{self, api::ExtensionInstall},
    storage,
};
use anyhow::{Context, Result, anyhow, bail};
use dropshot::{
    ClientErrorStatusCode, HttpError, HttpResponseOk, Path, RequestContext, TypedBody, endpoint,
};
use figment::providers::{Format, Toml};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};
use strum::Display;
use tracing::{error, info, warn};

/// Extensions every Gofer install gets unless the config turns them off with `enabled = false`, along with their
/// manifests. The manifests are built into Gofer and regenerated each release for the extension images built alongside
/// it (`make generate-manifests`), so a Gofer always runs the default extensions it was released with and starting them
/// never depends on fetching anything.
///
/// Gofer's optional extensions, like github, and any other extension are installed with a manifest URL or path
/// instead.
const DEFAULT_EXTENSIONS: [(&str, &str); 2] = [
    (
        "cron",
        include_str!("../../../../containers/extensions/cron/manifest.toml"),
    ),
    (
        "interval",
        include_str!("../../../../containers/extensions/interval/manifest.toml"),
    ),
];

pub fn default_manifest(name: &str) -> Option<&'static str> {
    DEFAULT_EXTENSIONS
        .iter()
        .find(|(default, _)| *default == name)
        .map(|(_, content)| *content)
}

/// The path Gofer serves a default extension's manifest at, so operators can read exactly what it's running.
fn default_manifest_path(name: &str) -> String {
    format!("/extensions/manifests/{name}.toml")
}

/// Describes an extension to Gofer: which image to run, which settings it takes, and which parameters pipelines pass
/// when they subscribe. It's the only place Gofer learns about an extension; the running extension is never asked.
/// Extension authors generate it with the SDK (`<extension binary> manifest --image <image>`) rather than writing it
/// by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Manifest {
    pub image: String,

    #[serde(default)]
    pub documentation: String,

    #[serde(default)]
    pub config_params: Vec<Parameter>,

    #[serde(default)]
    pub pipeline_subscription_params: Vec<Parameter>,
}

impl Manifest {
    pub fn parse(content: &str) -> Result<Self> {
        let manifest: Manifest = figment::Figment::from(Toml::string(content))
            .extract()
            .context("Could not parse manifest")?;

        if manifest.image.trim().is_empty() {
            bail!("Manifest is missing 'image'");
        }

        for (kind, params) in [
            ("config param", &manifest.config_params),
            (
                "pipeline subscription param",
                &manifest.pipeline_subscription_params,
            ),
        ] {
            let mut seen = HashSet::new();
            for param in params {
                if param.key.trim().is_empty() {
                    bail!("Manifest has a {kind} with an empty key");
                }
                if !seen.insert(param.key.as_str()) {
                    bail!("Manifest lists {kind} '{}' twice", param.key);
                }
            }
        }

        Ok(manifest)
    }

    pub fn to_documentation(&self) -> Documentation {
        Documentation {
            config_params: self.config_params.clone(),
            pipeline_subscription_params: self.pipeline_subscription_params.clone(),
            body: self.documentation.clone(),
        }
    }

    fn param(&self, key: &str) -> Option<&Parameter> {
        self.config_params.iter().find(|param| param.key == key)
    }
}

/// Reads a manifest from an https URL or a path on the server.
///
/// Plain http is refused: the manifest decides which image Gofer runs, so anyone who could tamper with it in
/// transit could get their own container run with the extension's token.
pub async fn fetch_manifest(location: &str) -> Result<Manifest> {
    let content = if location.starts_with("https://") {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?
            .get(location)
            .send()
            .await
            .and_then(|response| response.error_for_status())
            .with_context(|| format!("Could not download manifest from '{location}'"))?
            .text()
            .await
            .with_context(|| format!("Could not read manifest from '{location}'"))?
    } else if location.starts_with("http://") {
        bail!("Manifest '{location}' must be fetched over https, not http")
    } else {
        tokio::fs::read_to_string(location)
            .await
            .with_context(|| format!("Could not read manifest file '{location}'"))?
    };

    Manifest::parse(&content).with_context(|| format!("Manifest '{location}' is invalid"))
}

pub fn is_global_secret_ref(value: &str) -> bool {
    matches!(
        parse_interpolation_syntax(value),
        Some((InterpolationKind::GlobalSecret, _))
    )
}

/// An extension's settings and registry auth with their secret references swapped for the real values. These go to
/// the extension's container and nowhere else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolved {
    pub settings: BTreeMap<String, String>,
    pub registry_auth: Option<RegistryAuth>,
}

/// Everything needed to run an extension the way the config describes, worked out and checked before any container
/// is touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    pub manifest_location: String,
    pub manifest: Manifest,

    /// The registration as it gets stored; settings keep their secret references.
    pub registration: Registration,
    pub resolved: Resolved,

    /// A fingerprint of everything that affects the running container, resolved secrets included. When it differs
    /// from what's running, the extension needs a restart.
    pub hash: String,
}

/// Gofer's default extensions with the operator's entries layered on top by id.
///
/// The defaults live here rather than in the default config file because figment replaces arrays when merging
/// config files; listing them there would make them disappear the moment an operator added an entry of their own.
pub fn desired_extensions(installs: &[ExtensionInstall]) -> Result<Vec<ExtensionInstall>> {
    let mut desired: Vec<ExtensionInstall> = DEFAULT_EXTENSIONS
        .iter()
        .map(|(id, _)| ExtensionInstall {
            id: id.to_string(),
            manifest: None,
            enabled: true,
            settings: BTreeMap::new(),
            additional_roles: vec![],
            registry_auth: None,
        })
        .collect();

    let mut seen = HashSet::new();
    for install in installs {
        if !seen.insert(install.id.as_str()) {
            bail!(
                "Extension '{}' is listed more than once under [[extensions.install]]",
                install.id
            );
        }

        match desired.iter_mut().find(|entry| entry.id == install.id) {
            Some(entry) => *entry = install.clone(),
            None => desired.push(install.clone()),
        }
    }

    Ok(desired)
}

/// Checks an entry's settings against its manifest and fills in defaults. Every problem is reported at once so an
/// operator isn't stuck fixing them one reload at a time.
fn check_settings(
    manifest: &Manifest,
    settings: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut problems = vec![];
    let mut checked = BTreeMap::new();

    for key in settings.keys() {
        if manifest.param(key).is_none() {
            let valid: Vec<&str> = manifest
                .config_params
                .iter()
                .map(|param| param.key.as_str())
                .collect();
            problems.push(format!(
                "'{key}' is not a setting this extension takes; valid settings: {valid:?}"
            ));
        }
    }

    for param in &manifest.config_params {
        let value = match settings.get(&param.key) {
            Some(value) => value.clone(),
            None if !param.default.is_empty() => param.default.clone(),
            None => {
                if param.required {
                    problems.push(format!("'{}' is required but not set", param.key));
                }
                continue;
            }
        };

        if param.secret && !is_global_secret_ref(&value) {
            problems.push(format!(
                "'{}' is a secret, so it must be a global secret reference like \
                'global_secret{{{{{}}}}}' rather than the value itself",
                param.key,
                param.key.replace('_', "-")
            ));
            continue;
        }

        checked.insert(param.key.clone(), value);
    }

    if !problems.is_empty() {
        bail!("{}", problems.join("; "));
    }

    Ok(checked)
}

/// Swaps a global secret reference for the secret's value; anything else is passed through as is.
async fn resolve_value(api_state: &ApiState, value: &str) -> Result<String> {
    match parse_interpolation_syntax(value) {
        None => Ok(value.to_string()),
        Some((InterpolationKind::GlobalSecret, key)) => {
            fetch_global_secret(
                &api_state.storage,
                api_state.secret_store.as_ref(),
                &key,
                None,
            )
            .await
        }
        Some((kind, _)) => bail!(
            "'{value}' uses {kind} interpolation, but extension settings only support global secrets"
        ),
    }
}

fn fingerprint(prepared_image: &str, resolved: &Resolved, additional_roles: &[String]) -> String {
    let mut roles = additional_roles.to_vec();
    roles.sort();

    let fingerprint = serde_json::json!({
        "image": prepared_image,
        "settings": resolved.settings,
        "registry_auth": resolved.registry_auth,
        "additional_roles": roles,
    });

    hex::encode(Sha256::digest(fingerprint.to_string().as_bytes()))
}

/// Where the entry's manifest lives, and the manifest itself. Default extensions that leave `manifest` out use the copy
/// built into Gofer.
async fn load_manifest(install: &ExtensionInstall) -> Result<(String, Manifest)> {
    if let Some(location) = &install.manifest {
        let manifest = fetch_manifest(location).await?;
        return Ok((location.clone(), manifest));
    }

    let Some(content) = default_manifest(&install.id) else {
        bail!(
            "No manifest given; only Gofer's default extensions ({}) can leave 'manifest' out",
            DEFAULT_EXTENSIONS
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let manifest = Manifest::parse(content)
        .with_context(|| format!("Gofer's built-in manifest for '{}' is invalid", install.id))?;
    Ok((default_manifest_path(&install.id), manifest))
}

/// Works out everything needed to run an entry without touching any running extension: checks the settings against
/// the manifest and resolves the secrets.
async fn prepare(
    api_state: &ApiState,
    install: &ExtensionInstall,
    manifest_location: String,
    manifest: Manifest,
) -> Result<Prepared> {
    is_valid_identifier(&install.id)
        .with_context(|| format!("'{}' is not a valid extension id", install.id))?;

    let settings = check_settings(&manifest, &install.settings)?;

    let mut resolved = Resolved::default();
    for (key, value) in &settings {
        let value = resolve_value(api_state, value)
            .await
            .with_context(|| format!("Could not resolve setting '{key}'"))?;
        resolved.settings.insert(key.clone(), value);
    }

    let registry_auth = match &install.registry_auth {
        Some(auth) => {
            if !is_global_secret_ref(&auth.pass) {
                bail!(
                    "registry_auth.pass must be a global secret reference like 'global_secret{{{{registry-pass}}}}'"
                );
            }

            resolved.registry_auth = Some(RegistryAuth {
                user: auth.user.clone(),
                pass: resolve_value(api_state, &auth.pass)
                    .await
                    .context("Could not resolve registry_auth.pass")?,
            });

            Some(RegistryAuth {
                user: auth.user.clone(),
                pass: auth.pass.clone(),
            })
        }
        None => None,
    };

    let hash = fingerprint(&manifest.image, &resolved, &install.additional_roles);

    let registration = Registration {
        extension_id: install.id.clone(),
        image: manifest.image.clone(),
        registry_auth,
        settings: settings
            .into_iter()
            .map(|(key, value)| Variable {
                key,
                value,
                source: VariableSource::System,
            })
            .collect(),
        created: epoch_milli(),
        modified: epoch_milli(),
        status: Status::Enabled,
        additional_roles: install.additional_roles.clone(),
        key_id: String::new(),
    };

    Ok(Prepared {
        manifest_location,
        manifest,
        registration,
        resolved,
        hash,
    })
}

/// What the config says should happen to one extension.
enum Target {
    Run(Box<Prepared>),
    Disabled,
    Unconfigured,

    /// The config entry has a problem (bad manifest, missing setting, missing secret, ...). Whatever is running is
    /// left alone. The documentation is there whenever the manifest itself loaded.
    Invalid {
        manifest: String,
        documentation: Option<Documentation>,
        error: String,
    },
}

impl Target {
    /// Identifies the target so apply can tell whether anything changed since the operator looked at the plan.
    fn plan_hash(&self) -> String {
        match self {
            Target::Run(prepared) => prepared.hash.clone(),
            Target::Disabled => "disabled".into(),
            Target::Unconfigured => "unconfigured".into(),
            Target::Invalid { .. } => "invalid".into(),
        }
    }
}

async fn target_for(api_state: &ApiState, install: &ExtensionInstall) -> Target {
    if !install.enabled {
        return Target::Disabled;
    }

    let (location, manifest) = match load_manifest(install).await {
        Ok(loaded) => loaded,
        Err(e) => {
            return Target::Invalid {
                manifest: install.manifest.clone().unwrap_or_default(),
                documentation: None,
                error: format!("{e:#}"),
            };
        }
    };

    let documentation = manifest.to_documentation();
    match prepare(api_state, install, location.clone(), manifest).await {
        Ok(prepared) => Target::Run(Box::new(prepared)),
        Err(e) => Target::Invalid {
            manifest: location,
            documentation: Some(documentation),
            error: format!("{e:#}"),
        },
    }
}

/// The target for every extension the config mentions plus every extension Gofer knows about that the config no
/// longer mentions.
async fn targets(
    api_state: &ApiState,
    installs: &[ExtensionInstall],
) -> Result<Vec<(String, Target)>> {
    let desired = desired_extensions(installs)?;
    let mut targets = vec![];

    for install in &desired {
        targets.push((install.id.clone(), target_for(api_state, install).await));
    }

    let known: Vec<String> = api_state
        .extensions
        .iter()
        .map(|extension| extension.key().clone())
        .collect();

    for extension_id in known {
        if !desired.iter().any(|install| install.id == extension_id) {
            targets.push((extension_id, Target::Unconfigured));
        }
    }

    Ok(targets)
}

/// Reads Gofer's config file again so a reload sees the operator's latest edits. Environment variables are read
/// again too, but those can't change for a running process, so in practice it's the file that matters.
fn load_installs(api_state: &ApiState) -> Result<Vec<ExtensionInstall>> {
    let config = conf::Configuration::<conf::api::ApiConfig>::load(api_state.config_path.clone())
        .context("Could not read Gofer's configuration")?;
    Ok(config.extensions.install)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PlanAction {
    /// New to Gofer; it'll be registered and started.
    Install,

    /// Its config changed; the old container is stopped and a new one started.
    Update,

    /// Its config is unchanged but it isn't running, usually because it failed to start earlier.
    Start,

    /// Already running as the config describes.
    Unchanged,

    /// Turned off with `enabled = false`; it'll be stopped.
    Disable,

    /// No longer in the config; it'll be stopped but its subscriptions and data are kept.
    Unconfigure,

    /// The config entry has a problem. Nothing will be changed until it's fixed.
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FieldChange {
    pub field: String,
    pub old: String,
    pub new: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlannedExtension {
    pub extension_id: String,
    pub action: PlanAction,

    /// Where the extension's manifest comes from.
    pub manifest: String,

    /// What's different between what's running and what the config describes.
    pub changes: Vec<FieldChange>,

    /// Why the config entry is invalid. Empty unless the action is `invalid`.
    pub error: String,

    /// Pass this back when applying so Gofer can refuse if anything changed after the plan was made.
    pub plan_hash: String,
}

fn settings_map(registration: &Registration) -> BTreeMap<String, String> {
    registration
        .settings
        .iter()
        .map(|setting| (setting.key.clone(), setting.value.clone()))
        .collect()
}

/// The differences between the extension as Gofer has it and what the config describes.
fn diff(current: &Extension, prepared: &Prepared) -> Vec<FieldChange> {
    let mut changes = vec![];
    let old = &current.registration;
    let new = &prepared.registration;

    let mut change = |field: &str, old: String, new: String| {
        changes.push(FieldChange {
            field: field.into(),
            old,
            new,
        })
    };

    if old.status != Status::Enabled {
        change(
            "status",
            old.status.to_string(),
            Status::Enabled.to_string(),
        );
    }

    if !current.manifest.is_empty() && current.manifest != prepared.manifest_location {
        change(
            "manifest",
            current.manifest.clone(),
            prepared.manifest_location.clone(),
        );
    }

    if old.image != new.image {
        change("image", old.image.clone(), new.image.clone());
    }

    let old_settings = settings_map(old);
    let new_settings = settings_map(new);
    let applied_settings = current
        .applied
        .as_ref()
        .map(|applied| applied.resolved.settings.clone());

    for key in old_settings
        .keys()
        .chain(new_settings.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let field = format!("settings.{key}");
        match (old_settings.get(key), new_settings.get(key)) {
            (Some(old_value), Some(new_value)) if old_value != new_value => {
                change(&field, old_value.clone(), new_value.clone())
            }
            (Some(value), Some(_)) => {
                // Same reference, but the secret behind it may have been rotated.
                if let Some(applied) = &applied_settings
                    && applied.get(key) != prepared.resolved.settings.get(key)
                {
                    change(
                        &field,
                        value.clone(),
                        format!("{value} (secret value changed)"),
                    );
                }
            }
            (Some(old_value), None) => change(&field, old_value.clone(), String::new()),
            (None, Some(new_value)) => change(&field, String::new(), new_value.clone()),
            (None, None) => {}
        }
    }

    let mut old_roles = old.additional_roles.clone();
    let mut new_roles = new.additional_roles.clone();
    old_roles.sort();
    new_roles.sort();
    if old_roles != new_roles {
        change(
            "additional_roles",
            old_roles.join(", "),
            new_roles.join(", "),
        );
    }

    let show_auth = |registration: &Registration| {
        registration
            .registry_auth
            .as_ref()
            .map(|auth| format!("{} / {}", auth.user, auth.pass))
            .unwrap_or_default()
    };
    let old_auth = show_auth(old);
    let new_auth = show_auth(new);
    if old_auth != new_auth {
        change("registry_auth", old_auth, new_auth);
    } else if let Some(applied) = &current.applied
        && applied.resolved.registry_auth != prepared.resolved.registry_auth
    {
        change(
            "registry_auth",
            old_auth.clone(),
            format!("{old_auth} (secret value changed)"),
        );
    }

    changes
}

/// What applying a target would do to an extension, or `None` when there's nothing to say about it (a disabled
/// extension that was never installed).
fn plan_one(
    extension_id: &str,
    current: Option<&Extension>,
    target: &Target,
) -> Option<PlannedExtension> {
    let planned =
        |action: PlanAction, manifest: String, changes: Vec<FieldChange>, error: String| {
            Some(PlannedExtension {
                extension_id: extension_id.into(),
                action,
                manifest,
                changes,
                error,
                plan_hash: target.plan_hash(),
            })
        };
    // An extension that failed before it was ever registered is only a placeholder; as far as the plan goes it
    // isn't installed yet. Disabling or removing it still needs an apply so the placeholder gets cleared away.
    let placeholder = current.is_some_and(|extension| is_placeholder(&extension.registration));
    if placeholder {
        match target {
            Target::Disabled => {
                return planned(PlanAction::Disable, String::new(), vec![], String::new());
            }
            Target::Unconfigured => {
                return planned(
                    PlanAction::Unconfigure,
                    String::new(),
                    vec![],
                    String::new(),
                );
            }
            _ => {}
        }
    }
    let current = current.filter(|extension| !is_placeholder(&extension.registration));
    let running = current.is_some_and(|extension| extension.state == State::Running);
    let current_manifest = current
        .map(|extension| extension.manifest.clone())
        .unwrap_or_default();

    match target {
        Target::Run(prepared) => {
            let manifest = prepared.manifest_location.clone();
            let Some(current) = current else {
                let mut changes = vec![FieldChange {
                    field: "image".into(),
                    old: String::new(),
                    new: prepared.registration.image.clone(),
                }];
                for setting in &prepared.registration.settings {
                    changes.push(FieldChange {
                        field: format!("settings.{}", setting.key),
                        old: String::new(),
                        new: setting.value.clone(),
                    });
                }
                return planned(PlanAction::Install, manifest, changes, String::new());
            };

            let applied_hash = current
                .applied
                .as_ref()
                .map(|applied| applied.hash.as_str());
            if running && applied_hash == Some(prepared.hash.as_str()) {
                return planned(PlanAction::Unchanged, manifest, vec![], String::new());
            }

            let changes = diff(current, prepared);
            if !running && changes.is_empty() {
                return planned(PlanAction::Start, manifest, changes, String::new());
            }

            planned(PlanAction::Update, manifest, changes, String::new())
        }
        Target::Disabled => {
            let current = current?;
            if current.registration.status == Status::Disabled && !running {
                return planned(
                    PlanAction::Unchanged,
                    current_manifest,
                    vec![],
                    String::new(),
                );
            }
            planned(PlanAction::Disable, current_manifest, vec![], String::new())
        }
        Target::Unconfigured => {
            let current = current?;
            if current.registration.status == Status::Unconfigured && !running {
                return planned(
                    PlanAction::Unchanged,
                    current_manifest,
                    vec![],
                    String::new(),
                );
            }
            planned(
                PlanAction::Unconfigure,
                current_manifest,
                vec![],
                String::new(),
            )
        }
        Target::Invalid {
            manifest, error, ..
        } => planned(PlanAction::Invalid, manifest.clone(), vec![], error.clone()),
    }
}

/// Writes the registration to the database, registering the extension first if Gofer has never seen it.
async fn save_registration(api_state: &ApiState, registration: &Registration) -> Result<()> {
    let mut conn = api_state
        .storage
        .write_conn()
        .await
        .map_err(|e| anyhow!("Could not open connection to database; {:#?}", e))?;

    match storage::extension_registrations::get(&mut conn, &registration.extension_id).await {
        Ok(_) => {}
        Err(storage::StorageError::NotFound) => {
            drop(conn);
            return install_new_extension(api_state, registration).await;
        }
        Err(e) => bail!("Could not get extension registration; {:#?}", e),
    }

    let stored: storage::extension_registrations::ExtensionRegistration =
        registration.clone().try_into()?;

    storage::extension_registrations::update(
        &mut conn,
        &registration.extension_id,
        storage::extension_registrations::UpdatableFields {
            image: Some(stored.image),
            registry_auth: Some(stored.registry_auth),
            settings: Some(stored.settings),
            status: Some(stored.status),
            key_id: None,
            additional_roles: Some(stored.additional_roles),
            modified: epoch_milli().to_string(),
        },
    )
    .await
    .map_err(|e| anyhow!("Could not update extension registration; {:#?}", e))?;

    Ok(())
}

async fn set_status(api_state: &ApiState, extension_id: &str, status: &Status) -> Result<()> {
    let mut conn = api_state
        .storage
        .write_conn()
        .await
        .map_err(|e| anyhow!("Could not open connection to database; {:#?}", e))?;

    storage::extension_registrations::update(
        &mut conn,
        extension_id,
        storage::extension_registrations::UpdatableFields {
            status: Some(status.to_string()),
            ..Default::default()
        },
    )
    .await
    .map_err(|e| anyhow!("Could not update extension registration; {:#?}", e))
}

/// Stores the registration and starts the extension from it. On failure the extension is recorded as failed with
/// the reason, rather than the error being returned.
async fn run_prepared(
    api_state: &Arc<ApiState>,
    prepared: Prepared,
    previous: Option<Box<Prepared>>,
) -> Extension {
    let extension_id = prepared.registration.extension_id.clone();

    let failed = |reason: String, previous: Option<Box<Prepared>>| {
        error!(
            extension_id = extension_id,
            reason = reason,
            "Could not start extension"
        );
        let mut extension = Extension::not_running(
            prepared.registration.clone(),
            &prepared.manifest_location,
            State::Failed,
            &reason,
        );
        extension.documentation = prepared.manifest.to_documentation();
        extension.previous = previous;
        extension
    };

    if let Err(e) = save_registration(api_state, &prepared.registration).await {
        return failed(format!("{e:#}"), previous);
    }

    // start_extension replaces the extension's API token, so it needs the id of the current one to delete.
    let mut registration = prepared.registration.clone();
    if let Ok(mut conn) = api_state.storage.read_conn().await
        && let Ok(stored) = storage::extension_registrations::get(&mut conn, &extension_id).await
    {
        registration.key_id = stored.key_id;
    }

    match start_extension(api_state.clone(), registration, &prepared.resolved).await {
        Ok(mut extension) => {
            extension.manifest = prepared.manifest_location.clone();
            extension.documentation = prepared.manifest.to_documentation();
            extension.applied = Some(Box::new(prepared));
            extension.previous = previous;
            extension
        }
        Err(e) => {
            // The container may have come up without ever answering; don't leave it behind.
            stop_extension(api_state, &extension_id).await;
            failed(format!("{e:#}"), previous)
        }
    }
}

/// Brings one extension in line with its target and returns how it ended up.
async fn apply_target(api_state: &Arc<ApiState>, extension_id: &str, target: Target) -> Extension {
    let current = api_state
        .extensions
        .get(extension_id)
        .map(|extension| extension.value().clone());

    let extension = match target {
        Target::Run(prepared) => {
            let running = current
                .as_ref()
                .is_some_and(|extension| extension.state == State::Running);
            let previous = current
                .as_ref()
                .and_then(|extension| extension.applied.clone())
                .filter(|_| running);

            if running {
                stop_extension(api_state, extension_id).await;
            }

            run_prepared(api_state, *prepared, previous).await
        }
        Target::Disabled | Target::Unconfigured => {
            // Never registered, so there's nothing to stop or keep; just clear any placeholder away.
            let Some(current) =
                current.filter(|extension| !is_placeholder(&extension.registration))
            else {
                api_state.extensions.remove(extension_id);
                return Extension::not_running(
                    placeholder_registration(extension_id),
                    "",
                    State::Stopped,
                    "Not installed",
                );
            };

            let (status, reason, manifest) = match target {
                Target::Disabled => (
                    Status::Disabled,
                    "Disabled in Gofer's config",
                    current.manifest.clone(),
                ),
                _ => (
                    Status::Unconfigured,
                    "No longer in Gofer's config; 'gofer extension purge' deletes it for good",
                    String::new(),
                ),
            };

            stop_extension(api_state, extension_id).await;

            let disabled = status == Status::Disabled;
            let mut registration = current.registration.clone();
            if let Err(e) = set_status(api_state, extension_id, &status).await {
                error!(extension_id = extension_id, error = %e, "Could not update extension status");
            } else {
                registration.status = status;
            }

            let mut extension =
                Extension::not_running(registration, &manifest, State::Stopped, reason);
            if disabled {
                extension.documentation = current.documentation;
            }
            extension
        }
        Target::Invalid {
            manifest,
            documentation,
            error,
        } => {
            let registration = match current {
                // Leave a running extension alone; it keeps working with its old config until the entry is fixed.
                Some(current) if current.state == State::Running => {
                    api_state
                        .extensions
                        .insert(extension_id.to_string(), current.clone());
                    return current;
                }
                Some(current) => current.registration,
                None => placeholder_registration(extension_id),
            };

            let mut extension =
                Extension::not_running(registration, &manifest, State::Failed, &error);
            extension.documentation = documentation.unwrap_or_default();
            extension
        }
    };

    api_state
        .extensions
        .insert(extension_id.to_string(), extension.clone());

    extension
}

/// Real registrations always have a creation time, placeholders don't.
fn is_placeholder(registration: &Registration) -> bool {
    registration.created == 0
}

/// Stands in for an extension that's in the config but has never been registered, so failures still show up in
/// `gofer extension list`.
fn placeholder_registration(extension_id: &str) -> Registration {
    Registration {
        extension_id: extension_id.into(),
        image: String::new(),
        registry_auth: None,
        settings: vec![],
        created: 0,
        modified: 0,
        status: Status::Enabled,
        additional_roles: vec![],
        key_id: String::new(),
    }
}

/// Brings every extension in line with the config Gofer started with. Called once at startup; nothing here is
/// allowed to stop Gofer from starting, so problems are logged and recorded on the extension instead.
pub async fn reconcile_on_boot(api_state: Arc<ApiState>) {
    let _guard = api_state.extension_lock.lock().await;

    // Everything Gofer has registered starts out stopped; the targets below decide what runs.
    match api_state.storage.read_conn().await {
        Ok(mut conn) => match storage::extension_registrations::list(&mut conn).await {
            Ok(registrations) => {
                for stored in registrations {
                    let registration: Registration = match stored.try_into() {
                        Ok(registration) => registration,
                        Err(e) => {
                            error!(error = %e, "Could not parse extension registration");
                            continue;
                        }
                    };
                    api_state.extensions.insert(
                        registration.extension_id.clone(),
                        Extension::not_running(registration, "", State::Stopped, ""),
                    );
                }
            }
            Err(e) => error!(error = %e, "Could not list extension registrations"),
        },
        Err(e) => error!(error = %e, "Could not open connection to database"),
    }

    let targets = match targets(&api_state, &api_state.config.extensions.install).await {
        Ok(targets) => targets,
        Err(e) => {
            error!(error = %e, "Could not work out which extensions to run; no extensions were started");
            return;
        }
    };

    for (extension_id, target) in targets {
        let extension = apply_target(&api_state, &extension_id, target).await;
        match extension.state {
            State::Running => info!(extension_id = extension_id, "Extension running"),
            State::Failed => warn!(
                extension_id = extension_id,
                reason = extension.state_reason,
                "Extension failed to start; fix its config and run 'gofer extension reload'"
            ),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlanExtensionsResponse {
    /// Every extension Gofer knows about or the config mentions, and what applying the config would do to it.
    pub extensions: Vec<PlannedExtension>,
}

/// Compare Gofer's config with the extensions currently running.
///
/// Rereads Gofer's config file and reports, for each extension, what applying the config would change. Nothing is
/// changed. This backs `gofer extension reload`. This route is only accessible for admin tokens.
#[endpoint(
    method = POST,
    path = "/api/extensions/plan",
    tags = ["Extensions"],
)]
pub async fn plan_extensions(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<PlanExtensionsResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                allow_anonymous: false,
                requires: Requirement::Extension {
                    extension: None,
                    resource: None,
                },
                action: Action::Read,
            },
        )
        .await?;

    let installs =
        load_installs(api_state).map_err(|e| HttpError::for_bad_request(None, format!("{e:#}")))?;

    let targets = targets(api_state, &installs)
        .await
        .map_err(|e| HttpError::for_bad_request(None, format!("{e:#}")))?;

    let extensions = targets
        .iter()
        .filter_map(|(extension_id, target)| {
            let current = api_state
                .extensions
                .get(extension_id)
                .map(|extension| extension.value().clone());
            plan_one(extension_id, current.as_ref(), target)
        })
        .collect();

    Ok(HttpResponseOk(PlanExtensionsResponse { extensions }))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ApplyExtensionRequest {
    /// The `plan_hash` from the plan the operator reviewed.
    pub plan_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ApplyExtensionResponse {
    /// The extension as it ended up. Check `state` and `state_reason` to see whether it started.
    pub extension: Extension,

    /// Whether the version that was running before can be brought back with revert.
    pub can_revert: bool,
}

/// Bring one extension in line with Gofer's config.
///
/// Rereads Gofer's config file and installs, updates, starts, or stops the extension to match. The request carries
/// the `plan_hash` from the plan the operator reviewed; if anything changed since then (the config, the manifest,
/// or a secret) the request is refused so the operator can look at a fresh plan. This route is only accessible for
/// admin tokens.
#[endpoint(
    method = POST,
    path = "/api/extensions/{extension_id}/apply",
    tags = ["Extensions"],
)]
pub async fn apply_extension(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionPathArgs>,
    body: TypedBody<ApplyExtensionRequest>,
) -> Result<HttpResponseOk<ApplyExtensionResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let body = body.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                allow_anonymous: false,
                requires: Requirement::Extension {
                    extension: Some(path.extension_id.clone()),
                    resource: None,
                },
                action: Action::Write,
            },
        )
        .await?;

    let _guard = api_state.extension_lock.lock().await;

    let installs =
        load_installs(api_state).map_err(|e| HttpError::for_bad_request(None, format!("{e:#}")))?;
    let desired = desired_extensions(&installs)
        .map_err(|e| HttpError::for_bad_request(None, format!("{e:#}")))?;

    let target = match desired
        .iter()
        .find(|install| install.id == path.extension_id)
    {
        Some(install) => target_for(api_state, install).await,
        None if api_state.extensions.contains_key(&path.extension_id) => Target::Unconfigured,
        None => {
            return Err(HttpError::for_not_found(
                None,
                format!(
                    "Extension '{}' isn't in Gofer's config and isn't installed",
                    path.extension_id
                ),
            ));
        }
    };

    if target.plan_hash() != body.plan_hash {
        return Err(HttpError::for_client_error(
            None,
            ClientErrorStatusCode::CONFLICT,
            format!(
                "Extension '{}' changed since the plan was made (config, manifest, or a secret); make a new plan",
                path.extension_id
            ),
        ));
    }

    if let Target::Invalid { error, .. } = &target {
        return Err(HttpError::for_bad_request(
            None,
            format!(
                "Extension '{}' can't be applied; {error}",
                path.extension_id
            ),
        ));
    }

    let extension = apply_target(api_state, &path.extension_id, target).await;
    let can_revert = extension.previous.is_some();

    Ok(HttpResponseOk(ApplyExtensionResponse {
        extension,
        can_revert,
    }))
}

/// Go back to the version of an extension that was running before the last apply.
///
/// Meant for when a new version won't start. The extension stays different from Gofer's config until the config is
/// fixed, so it'll show up in the next plan again. This route is only accessible for admin tokens.
#[endpoint(
    method = POST,
    path = "/api/extensions/{extension_id}/revert",
    tags = ["Extensions"],
)]
pub async fn revert_extension(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<ExtensionPathArgs>,
) -> Result<HttpResponseOk<ApplyExtensionResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                allow_anonymous: false,
                requires: Requirement::Extension {
                    extension: Some(path.extension_id.clone()),
                    resource: None,
                },
                action: Action::Write,
            },
        )
        .await?;

    let _guard = api_state.extension_lock.lock().await;

    let previous = api_state
        .extensions
        .get(&path.extension_id)
        .and_then(|extension| extension.previous.clone())
        .ok_or_else(|| {
            HttpError::for_bad_request(
                None,
                format!(
                    "Extension '{}' has no earlier version to revert to",
                    path.extension_id
                ),
            )
        })?;

    stop_extension(api_state, &path.extension_id).await;
    let extension = run_prepared(api_state, *previous, None).await;
    api_state
        .extensions
        .insert(path.extension_id.clone(), extension.clone());

    Ok(HttpResponseOk(ApplyExtensionResponse {
        extension,
        can_revert: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(id: &str) -> ExtensionInstall {
        ExtensionInstall {
            id: id.into(),
            manifest: Some(format!("https://example.com/{id}.toml")),
            enabled: true,
            settings: BTreeMap::new(),
            additional_roles: vec![],
            registry_auth: None,
        }
    }

    fn param(key: &str, required: bool, secret: bool, default: &str) -> Parameter {
        Parameter {
            key: key.into(),
            required,
            documentation: String::new(),
            secret,
            default: default.into(),
        }
    }

    fn manifest(params: Vec<Parameter>) -> Manifest {
        Manifest {
            image: "example.com/ext:1.0.0".into(),
            documentation: String::new(),
            config_params: params,
            pipeline_subscription_params: vec![],
        }
    }

    fn settings(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn defaults_are_included_and_overridable_by_id() {
        let mut cron = install("cron");
        cron.enabled = false;

        let desired = desired_extensions(&[cron, install("github")]).unwrap();
        let ids: Vec<&str> = desired.iter().map(|install| install.id.as_str()).collect();
        assert_eq!(ids, vec!["cron", "interval", "github"]);
        assert!(!desired[0].enabled);
        assert!(desired[1].enabled);
        assert!(desired[1].manifest.is_none());
    }

    #[test]
    fn default_manifests_are_valid() {
        for (name, content) in DEFAULT_EXTENSIONS {
            let manifest = Manifest::parse(content)
                .unwrap_or_else(|e| panic!("built-in manifest for {name} is invalid: {e:#}"));
            assert!(manifest.image.contains(name));
        }
    }

    #[tokio::test]
    async fn default_extensions_can_leave_out_their_manifest() {
        let mut interval = install("interval");
        interval.manifest = None;
        let (location, manifest) = load_manifest(&interval).await.unwrap();
        assert_eq!(location, "/extensions/manifests/interval.toml");
        assert!(manifest.image.contains("interval"));

        // github ships with Gofer too, but it's optional, so it needs a manifest like any other extension.
        let mut github = install("github");
        github.manifest = None;
        let err = load_manifest(&github).await.unwrap_err();
        assert!(err.to_string().contains("only Gofer's default extensions"));
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        assert!(desired_extensions(&[install("github"), install("github")]).is_err());
    }

    #[test]
    fn missing_required_settings_are_reported() {
        let manifest = manifest(vec![param("app_id", true, false, "")]);
        let err = check_settings(&manifest, &settings(&[])).unwrap_err();
        assert!(err.to_string().contains("'app_id' is required"));
    }

    #[test]
    fn unknown_settings_are_reported() {
        let manifest = manifest(vec![param("app_id", false, false, "")]);
        let err = check_settings(&manifest, &settings(&[("app_idd", "1")])).unwrap_err();
        assert!(err.to_string().contains("'app_idd' is not a setting"));
    }

    #[test]
    fn raw_values_for_secret_params_are_rejected() {
        let manifest = manifest(vec![param("app_key", true, true, "")]);
        let err = check_settings(&manifest, &settings(&[("app_key", "hunter2")])).unwrap_err();
        assert!(
            err.to_string()
                .contains("must be a global secret reference")
        );

        let checked = check_settings(
            &manifest,
            &settings(&[("app_key", "global_secret{{github-app-key}}")]),
        )
        .unwrap();
        assert_eq!(checked["app_key"], "global_secret{{github-app-key}}");
    }

    #[test]
    fn defaults_fill_in_unset_settings() {
        let manifest = manifest(vec![
            param("min_interval", false, false, "1m"),
            param("optional", false, false, ""),
        ]);
        let checked = check_settings(&manifest, &settings(&[])).unwrap();
        assert_eq!(checked.get("min_interval").map(String::as_str), Some("1m"));
        assert!(!checked.contains_key("optional"));

        let checked = check_settings(&manifest, &settings(&[("min_interval", "5m")])).unwrap();
        assert_eq!(checked["min_interval"], "5m");
    }

    #[test]
    fn manifest_parses_generated_output() {
        let manifest = Manifest::parse(
            r#"
image = "ghcr.io/clintjedwards/gofer/extensions/interval:0.11.0"
documentation = "docs"

[[config_params]]
key = "min_interval"
required = false
secret = false
default = "1m"
documentation = "The minimum interval."

[[pipeline_subscription_params]]
key = "every"
required = true
secret = false
default = ""
documentation = "Time between runs."
"#,
        )
        .unwrap();
        assert_eq!(manifest.config_params[0].default, "1m");

        let documentation = manifest.to_documentation();
        assert_eq!(documentation.body, "docs");
        assert_eq!(documentation.pipeline_subscription_params[0].key, "every");

        assert!(Manifest::parse("documentation = \"no image\"").is_err());
        assert!(
            Manifest::parse(
                "image = \"x\"\n[[pipeline_subscription_params]]\nkey = \"a\"\nrequired = true\ndocumentation = \"\"\n[[pipeline_subscription_params]]\nkey = \"a\"\nrequired = true\ndocumentation = \"\""
            )
            .is_err()
        );
    }

    fn prepared(image: &str, settings: &[(&str, &str)], resolved: &[(&str, &str)]) -> Prepared {
        let manifest = Manifest {
            image: image.into(),
            documentation: String::new(),
            pipeline_subscription_params: vec![],
            config_params: vec![
                param("app_key", true, true, ""),
                param("app_id", true, false, ""),
            ],
        };
        let resolved = Resolved {
            settings: self::settings(resolved),
            registry_auth: None,
        };
        let hash = fingerprint(image, &resolved, &[]);
        let mut registration = placeholder_registration("github");
        registration.created = 1;
        registration.image = image.into();
        registration.settings = settings
            .iter()
            .map(|(key, value)| Variable {
                key: key.to_string(),
                value: value.to_string(),
                source: VariableSource::System,
            })
            .collect();

        Prepared {
            manifest_location: "https://example.com/github.toml".into(),
            manifest,
            registration,
            resolved,
            hash,
        }
    }

    fn running(prepared: &Prepared) -> Extension {
        let mut extension = Extension::not_running(
            prepared.registration.clone(),
            &prepared.manifest_location,
            State::Running,
            "",
        );
        extension.applied = Some(Box::new(prepared.clone()));
        extension
    }

    #[test]
    fn plan_actions() {
        let refs = [("app_key", "global_secret{{key}}"), ("app_id", "1")];
        let current = prepared("img:1", &refs, &[("app_key", "a"), ("app_id", "1")]);
        let running = running(&current);

        // Nothing installed yet.
        let target = Target::Run(Box::new(current.clone()));
        assert_eq!(
            plan_one("github", None, &target).unwrap().action,
            PlanAction::Install
        );

        // Failed before it was ever registered; still an install, and removing it clears the placeholder.
        let placeholder = Extension::not_running(
            placeholder_registration("github"),
            "",
            State::Failed,
            "missing secret",
        );
        assert_eq!(
            plan_one("github", Some(&placeholder), &target)
                .unwrap()
                .action,
            PlanAction::Install
        );
        assert_eq!(
            plan_one("github", Some(&placeholder), &Target::Unconfigured)
                .unwrap()
                .action,
            PlanAction::Unconfigure
        );

        // Running exactly as configured.
        assert_eq!(
            plan_one("github", Some(&running), &target).unwrap().action,
            PlanAction::Unchanged
        );

        // Same config but not running, like after a failed start.
        let mut failed = running.clone();
        failed.state = State::Failed;
        failed.applied = None;
        assert_eq!(
            plan_one("github", Some(&failed), &target).unwrap().action,
            PlanAction::Start
        );

        // New image.
        let upgraded = Target::Run(Box::new(prepared(
            "img:2",
            &refs,
            &[("app_key", "a"), ("app_id", "1")],
        )));
        let plan = plan_one("github", Some(&running), &upgraded).unwrap();
        assert_eq!(plan.action, PlanAction::Update);
        assert_eq!(plan.changes[0].field, "image");

        // Same reference, rotated secret.
        let rotated = Target::Run(Box::new(prepared(
            "img:1",
            &refs,
            &[("app_key", "b"), ("app_id", "1")],
        )));
        let plan = plan_one("github", Some(&running), &rotated).unwrap();
        assert_eq!(plan.action, PlanAction::Update);
        assert_eq!(plan.changes.len(), 1);
        assert!(plan.changes[0].new.contains("secret value changed"));

        // Turned off, and removed from the config.
        assert_eq!(
            plan_one("github", Some(&running), &Target::Disabled)
                .unwrap()
                .action,
            PlanAction::Disable
        );
        assert!(plan_one("github", None, &Target::Disabled).is_none());
        assert_eq!(
            plan_one("github", Some(&running), &Target::Unconfigured)
                .unwrap()
                .action,
            PlanAction::Unconfigure
        );
    }
}
