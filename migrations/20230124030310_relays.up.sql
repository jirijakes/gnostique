-- Relay information.
CREATE TABLE "relays" (
       -- Relay URL.
       url TEXT NOT NULL PRIMARY KEY,
       -- Information JSON object.
       information TEXT NULL,
       -- Timestamp of last update.
       updated TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
