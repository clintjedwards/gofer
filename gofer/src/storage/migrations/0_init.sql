-- This table exists to act as a replacement for not being able to make a Transaction BEGIN IMMEDIATE.
-- Read storage/mod.rs for more.
CREATE TABLE IF NOT EXISTS transaction_mutex (
    id          TEXT    NOT NULL,
    lock        INTEGER NOT NULL CHECK (lock IN (0, 1))
) STRICT;

CREATE TABLE IF NOT EXISTS namespaces (
    id          TEXT    NOT NULL,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL,
    created     TEXT    NOT NULL,
    modified    TEXT    NOT NULL,
    PRIMARY KEY (id)
) STRICT;

CREATE TABLE IF NOT EXISTS pipeline_metadata (
    namespace_id TEXT    NOT NULL,
    pipeline_id  TEXT    NOT NULL,
    created      TEXT    NOT NULL,
    modified     TEXT    NOT NULL,
    state        TEXT    NOT NULL,
    FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE,
    PRIMARY KEY (namespace_id, pipeline_id)
) STRICT;

CREATE INDEX idx_created_pipeline_metadata ON pipeline_metadata (created);

CREATE TABLE IF NOT EXISTS pipeline_configs (
    namespace_id TEXT    NOT NULL,
    pipeline_id  TEXT    NOT NULL,
    version      INTEGER NOT NULL,
    parallelism  INTEGER NOT NULL,
    name         TEXT    NOT NULL,
    description  TEXT    NOT NULL,
    registered   TEXT    NOT NULL,
    deprecated   TEXT    NOT NULL,
    state        TEXT    NOT NULL,
    FOREIGN KEY (namespace_id, pipeline_id) REFERENCES pipeline_metadata(namespace_id, pipeline_id) ON DELETE CASCADE,
    PRIMARY KEY (namespace_id, pipeline_id, version)
) STRICT;

CREATE TABLE IF NOT EXISTS deployments (
    namespace_id  TEXT    NOT NULL,
    pipeline_id   TEXT    NOT NULL,
    deployment_id INTEGER NOT NULL,
    start_version INTEGER NOT NULL,
    end_version   INTEGER NOT NULL,
    started       TEXT    NOT NULL,
    ended         TEXT    NOT NULL,
    state         TEXT    NOT NULL,
    status        TEXT    NOT NULL,
    status_reason TEXT    NOT NULL,
    logs          TEXT    NOT NULL,
    FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id) REFERENCES pipeline_metadata(namespace_id, pipeline_id) ON DELETE CASCADE,
    PRIMARY KEY (namespace_id, pipeline_id, deployment_id)
) STRICT;

CREATE TABLE IF NOT EXISTS extension_subscriptions (
    namespace_id              TEXT NOT NULL,
    pipeline_id               TEXT NOT NULL,
    extension_id              TEXT NOT NULL,
    extension_subscription_id TEXT NOT NULL,
    settings                  TEXT NOT NULL,
    status                    TEXT NOT NULL,
    status_reason             TEXT NOT NULL,
    FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id) REFERENCES pipeline_metadata(namespace_id, pipeline_id) ON DELETE CASCADE,
    FOREIGN KEY (extension_id) REFERENCES extension_registrations(extension_id) ON DELETE CASCADE,
    PRIMARY KEY (namespace_id, pipeline_id, extension_id, extension_subscription_id)
) STRICT;

CREATE TABLE IF NOT EXISTS runs (
    namespace_id            TEXT    NOT NULL,
    pipeline_id             TEXT    NOT NULL,
    pipeline_config_version INTEGER NOT NULL,
    run_id                  INTEGER NOT NULL,
    started                 TEXT    NOT NULL,
    ended                   TEXT    NOT NULL,
    state                   TEXT    NOT NULL,
    status                  TEXT    NOT NULL,
    status_reason           TEXT    NOT NULL,
    initiator               TEXT    NOT NULL,
    variables               TEXT    NOT NULL,
    token_id                TEXT,
    store_objects_expired   INTEGER NOT NULL CHECK (store_objects_expired IN (0, 1)),
    event_id                TEXT,
    FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id) REFERENCES pipeline_metadata(namespace_id, pipeline_id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id, pipeline_config_version) REFERENCES pipeline_configs(namespace_id, pipeline_id, version),
    PRIMARY KEY (namespace_id, pipeline_id, run_id)
) STRICT;

CREATE TABLE IF NOT EXISTS tasks (
    namespace_id             TEXT    NOT NULL,
    pipeline_id              TEXT    NOT NULL,
    pipeline_config_version  INTEGER NOT NULL,
    task_id                  TEXT    NOT NULL,
    description              TEXT    NOT NULL,
    image                    TEXT    NOT NULL,
    registry_auth            TEXT    NOT NULL,
    depends_on               TEXT    NOT NULL,
    variables                TEXT    NOT NULL,
    entrypoint               TEXT    NOT NULL,
    command                  TEXT    NOT NULL,
    inject_api_token         INTEGER NOT NULL CHECK (inject_api_token IN (0, 1)),
    always_pull_newest_image INTEGER NOT NULL CHECK (always_pull_newest_image IN (0, 1)),
    FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id) REFERENCES pipeline_metadata(namespace_id, pipeline_id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id, pipeline_config_version) REFERENCES pipeline_configs(namespace_id, pipeline_id, version) ON DELETE CASCADE,
    PRIMARY KEY (namespace_id, pipeline_id, pipeline_config_version, task_id)
) STRICT;

CREATE TABLE IF NOT EXISTS extension_registrations (
    extension_id     TEXT NOT NULL,
    image            TEXT NOT NULL,
    registry_auth    TEXT NOT NULL,
    settings         TEXT NOT NULL,
    created          TEXT NOT NULL,
    modified         TEXT NOT NULL,
    status           TEXT NOT NULL,
    key_id           TEXT NOT NULL,
    additional_roles TEXT NOT NULL,
    PRIMARY KEY (extension_id)
) STRICT;

CREATE TABLE IF NOT EXISTS object_store_pipeline_keys(
    namespace_id  TEXT     NOT NULL,
    pipeline_id   TEXT     NOT NULL,
    key           TEXT     NOT NULL,
    created       TEXT     NOT NULL,
    FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id) REFERENCES pipeline_metadata(namespace_id, pipeline_id) ON DELETE CASCADE,
    PRIMARY KEY (namespace_id, pipeline_id, key)
) STRICT;

CREATE INDEX idx_object_pipeline_keys_created ON object_store_pipeline_keys (created);

CREATE TABLE IF NOT EXISTS object_store_run_keys(
    namespace_id  TEXT    NOT NULL,
    pipeline_id   TEXT    NOT NULL,
    run_id        INTEGER NOT NULL,
    key           TEXT    NOT NULL,
    created       TEXT    NOT NULL,
    FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id) REFERENCES pipeline_metadata(namespace_id, pipeline_id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id, run_id) REFERENCES runs(namespace_id, pipeline_id, run_id) ON DELETE CASCADE,
    PRIMARY KEY (namespace_id, pipeline_id, run_id, key)
) STRICT;

CREATE INDEX idx_object_run_keys_created ON object_store_run_keys (created);

CREATE TABLE IF NOT EXISTS object_store_extension_keys(
    extension_id  TEXT    NOT NULL,
    key           TEXT    NOT NULL,
    created       TEXT    NOT NULL,
    FOREIGN KEY (extension_id) REFERENCES extension_registrations(extension_id) ON DELETE CASCADE,
    PRIMARY KEY (extension_id, key)
) STRICT;

CREATE INDEX idx_object_extension_keys_created ON object_store_extension_keys (created);

CREATE TABLE IF NOT EXISTS secret_store_pipeline_keys(
    namespace_id  TEXT    NOT NULL,
    pipeline_id   TEXT    NOT NULL,
    key           TEXT    NOT NULL,
    created       TEXT    NOT NULL,
    FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id) REFERENCES pipeline_metadata(namespace_id, pipeline_id) ON DELETE CASCADE,
    PRIMARY KEY (namespace_id, pipeline_id, key)
) STRICT;

CREATE INDEX idx_secret_pipeline_keys_created ON secret_store_pipeline_keys (created);

CREATE TABLE IF NOT EXISTS secret_store_global_keys(
    key           TEXT    NOT NULL,
    namespaces    TEXT    NOT NULL,
    created       TEXT    NOT NULL,
    PRIMARY KEY (key)
) STRICT;

CREATE INDEX idx_secret_global_keys_created ON secret_store_global_keys (created);

CREATE TABLE IF NOT EXISTS task_executions (
    namespace_id      TEXT    NOT NULL,
    pipeline_id       TEXT    NOT NULL,
    run_id            INTEGER NOT NULL,
    task_id           TEXT    NOT NULL,
    task              TEXT    NOT NULL,
    created           TEXT    NOT NULL,
    started           TEXT    NOT NULL,
    ended             TEXT    NOT NULL,
    exit_code         INTEGER,
    logs_expired      INTEGER NOT NULL CHECK (logs_expired IN (0, 1)),
    logs_removed      INTEGER NOT NULL CHECK (logs_removed IN (0, 1)),
    state             TEXT    NOT NULL,
    status            TEXT    NOT NULL,
    status_reason     TEXT    NOT NULL,
    variables         TEXT    NOT NULL,
    FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id) REFERENCES pipeline_metadata(namespace_id, pipeline_id) ON DELETE CASCADE,
    FOREIGN KEY (namespace_id, pipeline_id, run_id) REFERENCES runs(namespace_id, pipeline_id, run_id) ON DELETE CASCADE,
    PRIMARY KEY (namespace_id, pipeline_id, run_id, task_id)
) STRICT;

CREATE INDEX idx_taskexecutions_started ON task_executions (started);

CREATE TABLE IF NOT EXISTS events (
    id       TEXT    NOT NULL,
    kind     TEXT    NOT NULL,
    details  TEXT    NOT NULL,
    emitted  TEXT    NOT NULL,
    PRIMARY KEY (id)
) STRICT;

CREATE TABLE IF NOT EXISTS tokens (
    id          TEXT    NOT NULL,
    hash        TEXT    NOT NULL,
    created     TEXT    NOT NULL,
    metadata    TEXT    NOT NULL,
    expires     TEXT    NOT NULL,
    disabled    INTEGER NOT NULL CHECK (disabled IN (0, 1)),
    roles       TEXT    NOT NULL,
    user        TEXT    NOT NULL
) STRICT;

CREATE INDEX idx_tokens_hash ON tokens (hash);

CREATE TABLE IF NOT EXISTS roles (
    id          TEXT    NOT NULL,
    description TEXT    NOT NULL,
    permissions TEXT    NOT NULL,
    system_role INTEGER NOT NULL CHECK (system_role IN (0, 1)),
    PRIMARY KEY (id)
) STRICT;

CREATE TABLE IF NOT EXISTS system (
    id                         INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    bootstrap_token_created    INTEGER NOT NULL CHECK (bootstrap_token_created IN (0, 1)),
    ignore_pipeline_run_events INTEGER NOT NULL CHECK (ignore_pipeline_run_events IN (0, 1))
) STRICT;

INSERT INTO system (id, bootstrap_token_created, ignore_pipeline_run_events) VALUES (1, 0, 0) ON CONFLICT(id) DO NOTHING;
