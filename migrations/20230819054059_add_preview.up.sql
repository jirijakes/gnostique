-- Link previews
CREATE TABLE previews (
  -- HTTP URL of the link
  url TEXT PRIMARY KEY NOT NULL ON CONFLICT REPLACE,
  -- Kind of content
  kind TEXT NOT NULL,
  -- Title of content
  title TEXT NULL,
  -- Description of content
  description TEXT NULL,
  -- Thumbnail (image)
  thumbnail BLOB NULL,
  -- Error during obtaining content
  error TEXT NULL,
  -- Time of obtaining content
  time TEXT NOT NULL DEFAULT (datetime('now'))
);
