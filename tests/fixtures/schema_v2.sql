CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS frameworks (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    built_in INTEGER NOT NULL CHECK (built_in IN (0, 1)),
    enabled_by_default INTEGER NOT NULL CHECK (enabled_by_default IN (0, 1)),
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS framework_mappings (
    id TEXT PRIMARY KEY NOT NULL,
    framework_id TEXT NOT NULL,
    source_path TEXT NOT NULL,
    link_path TEXT NOT NULL,
    link_kind TEXT NOT NULL CHECK (link_kind IN ('file', 'directory')),
    required INTEGER NOT NULL CHECK (required IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (framework_id) REFERENCES frameworks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS linkable_items (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('skill', 'resource')),
    name TEXT NOT NULL,
    alias TEXT,
    source_path TEXT NOT NULL,
    source_kind TEXT NOT NULL CHECK (source_kind IN ('file', 'directory')),
    target_dir TEXT,
    description TEXT,
    version TEXT,
    commit_hash TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_linkable_items_kind_name
    ON linkable_items(kind, name);
CREATE TABLE IF NOT EXISTS groups (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS group_items (
    group_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (group_id, item_id),
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES linkable_items(id) ON DELETE CASCADE
);
PRAGMA user_version = 1;
ALTER TABLE linkable_items
    ADD COLUMN source_type TEXT NOT NULL DEFAULT 'local-path';
ALTER TABLE linkable_items
    ADD COLUMN source_ownership TEXT NOT NULL DEFAULT 'external';
ALTER TABLE linkable_items
    ADD COLUMN repo_url TEXT;
ALTER TABLE linkable_items
    ADD COLUMN repo_commit TEXT;
PRAGMA user_version = 2;
