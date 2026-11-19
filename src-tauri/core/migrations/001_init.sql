-- Judgement schema v1: three tables (§5). IDs are ULIDs, timestamps UTC
-- RFC3339, list fields JSON arrays validated by the core library.

CREATE TABLE settings (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  participation TEXT NOT NULL CHECK (participation IN ('active', 'paused')),
  auto_learn    TEXT NOT NULL CHECK (auto_learn IN ('off', 'cool', 'warm', 'hot')),
  analyzer      TEXT NOT NULL CHECK (json_valid(analyzer)),
  updated_at    TEXT NOT NULL
);

CREATE TABLE items (
  id               TEXT PRIMARY KEY,
  title            TEXT NOT NULL,
  summary          TEXT NOT NULL,
  guidance         TEXT NOT NULL,
  applies_when     TEXT NOT NULL DEFAULT '',
  exceptions       TEXT NOT NULL DEFAULT '',
  tags             TEXT NOT NULL DEFAULT '[]' CHECK (json_type(tags) = 'array'),
  phases           TEXT NOT NULL DEFAULT '[]' CHECK (json_type(phases) = 'array'),
  repo_root        TEXT,
  status           TEXT NOT NULL CHECK (status IN ('active', 'omitted')),
  omitted_at       TEXT,
  source           TEXT NOT NULL CHECK (source IN ('manual', 'learn', 'auto')),
  source_harness   TEXT NOT NULL,
  source_model     TEXT NOT NULL,
  evidence         TEXT NOT NULL DEFAULT '[]' CHECK (json_type(evidence) = 'array'),
  times_returned   INTEGER NOT NULL DEFAULT 0,
  last_returned_at TEXT,
  reinforced_count INTEGER NOT NULL DEFAULT 0,
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL
);

CREATE INDEX items_status ON items (status);

CREATE TABLE learnings (
  id            TEXT PRIMARY KEY,
  source        TEXT NOT NULL CHECK (source IN ('manual', 'learn', 'auto')),
  kind          TEXT NOT NULL CHECK (kind IN ('new', 'reinforce', 'correct')),
  target_id     TEXT,
  payload       TEXT NOT NULL CHECK (json_valid(payload)),
  evidence      TEXT NOT NULL,
  harness       TEXT NOT NULL,
  model         TEXT NOT NULL,
  session_model TEXT,
  session_ref   TEXT,
  signal_score  INTEGER CHECK (signal_score BETWEEN 1 AND 4),
  temperature   TEXT CHECK (temperature IN ('off', 'cool', 'warm', 'hot')),
  status        TEXT NOT NULL CHECK (status IN ('pending', 'confirmed', 'denied')),
  created_at    TEXT NOT NULL,
  decided_at    TEXT
);

CREATE INDEX learnings_status ON learnings (status, created_at);
