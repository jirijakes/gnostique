-- Text notes.
CREATE TABLE textnotes (
       -- Event id.
       id BLOB PRIMARY KEY ON CONFLICT IGNORE,
       -- Original event JSON.
       event TEXT NOT NULL
);

-- m-to-n relationship between text notes and relays.
CREATE TABLE textnotes_relays (
       -- Text note's event id.
       textnote BLOB NOT NULL,
       -- Relay URL.
       relay TEXT NOT NULL,
       PRIMARY KEY (textnote, relay) ON CONFLICT IGNORE
);
