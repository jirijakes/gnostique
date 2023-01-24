-- Kind 0 events.
CREATE TABLE "metadata" (
       -- Pubkey of the sender of the metadata.
       author BLOB PRIMARY KEY,
       -- The complete event as JSON.
       event TEXT NOT NULL,
       -- Date and time of latest NIP05 verification. NULL if never performed.
       nip05_verified TEXT NULL DEFAULT NULL
);
