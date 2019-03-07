PRAGMA foreign_keys = 1;

CREATE TABLE modules (
	name TEXT NOT NULL PRIMARY KEY,
	enabled BOOL NOT NULL
);
INSERT INTO modules (name, enabled) VALUES ('core', true);

CREATE TABLE permissions (
	nick TEXT NOT NULL,
	user TEXT NOT NULL,
	host TEXT NOT NULL,
	flags INTEGER NOT NULL
);

CREATE TABLE config (
	id INTEGER PRIMARY KEY CHECK (id = 0), -- enforce single-row config
	cmdchars TEXT NOT NULL
);
INSERT INTO config (id, cmdchars) VALUES (0, '.');
