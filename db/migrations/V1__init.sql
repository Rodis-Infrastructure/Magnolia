-- create
CREATE TABLE highlight
(
    user_id  TEXT NOT NULL,
    guild_id TEXT NOT NULL,

    PRIMARY KEY (user_id, guild_id)
);

CREATE TABLE highlight_pattern
(
    user_id  TEXT NOT NULL,
    guild_id TEXT NOT NULL,
    pattern  TEXT NOT NULL,
    type     TEXT NOT NULL, -- "regex", "wildcard", "exact"

    PRIMARY KEY (user_id, guild_id),
    UNIQUE (user_id, guild_id, pattern),
    FOREIGN KEY (user_id, guild_id)
        REFERENCES highlight (user_id, guild_id)
        ON DELETE CASCADE
);

CREATE TABLE highlight_channel_scoping
(
    user_id    TEXT NOT NULL,
    guild_id   TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    type       TEXT NOT NULL, -- "whitelist", "blacklist"

    PRIMARY KEY (user_id, guild_id, channel_id),
    FOREIGN KEY (user_id, guild_id)
        REFERENCES highlight (user_id, guild_id)
        ON DELETE CASCADE
);
