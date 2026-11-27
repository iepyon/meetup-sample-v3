CREATE TABLE IF NOT EXISTS accounts (
  id            TEXT NOT NULL PRIMARY KEY,
  name          TEXT NOT NULL,
  email         TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS communities (
  id          TEXT NOT NULL PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  description TEXT,
  category    TEXT NOT NULL CHECK (category IN ('TECH', 'BUSINESS', 'HOBBY')),
  visibility  TEXT NOT NULL CHECK (visibility IN ('PUBLIC', 'PRIVATE')),
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS community_members (
  id           TEXT NOT NULL PRIMARY KEY,
  community_id TEXT NOT NULL REFERENCES communities (id),
  account_id   TEXT NOT NULL REFERENCES accounts (id),
  role         TEXT NOT NULL CHECK (role IN ('OWNER', 'ADMIN', 'MEMBER')),
  status       TEXT NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING', 'ACTIVE')),
  created_at   TEXT NOT NULL,
  UNIQUE (community_id, account_id)
);

CREATE INDEX IF NOT EXISTS idx_members_account ON community_members (account_id);

CREATE INDEX IF NOT EXISTS idx_members_community_status ON community_members (community_id, status, created_at);

CREATE TABLE IF NOT EXISTS events (
  id           TEXT NOT NULL PRIMARY KEY,
  community_id TEXT NOT NULL REFERENCES communities (id),
  created_by   TEXT NOT NULL REFERENCES accounts (id),
  title        TEXT NOT NULL,
  description  TEXT,
  starts_at    TEXT NOT NULL,
  ends_at      TEXT NOT NULL,
  format       TEXT NOT NULL CHECK (format IN ('ONLINE', 'OFFLINE', 'HYBRID')),
  capacity     INTEGER NOT NULL,
  status       TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'PUBLISHED', 'CLOSED', 'CANCELLED')),
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_community_status ON events (community_id, status)
