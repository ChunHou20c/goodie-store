-- Accounts and sessions. Two roles: everyone is a `user`; an `admin` may also
-- reach the admin console (see `import_products`).
create type user_role as enum ('user', 'admin');

create table users (
    id            bigint      generated always as identity primary key,
    email         text        not null unique,   -- always stored lowercased
    password_hash text        not null,          -- argon2id PHC string
    display_name  text,
    role          user_role   not null default 'user',
    created_at    timestamptz not null default now()
);

-- Opaque bearer tokens. Only the sha256 of a token is stored, so a dump of this
-- table cannot be replayed as a live session.
create table sessions (
    token_hash text        primary key,
    user_id    bigint      not null references users (id) on delete cascade,
    expires_at timestamptz not null,
    created_at timestamptz not null default now()
);

create index sessions_user_id_idx on sessions (user_id);
create index sessions_expires_at_idx on sessions (expires_at);
