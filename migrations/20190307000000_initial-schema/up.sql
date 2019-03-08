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
	flags INTEGER NOT NULL,
	PRIMARY KEY (nick, user, host)
);
INSERT INTO permissions VALUES ('GinjaNinja32', 'nyx', 'gn32.uk', 1);

CREATE TABLE config (
	id INTEGER PRIMARY KEY CHECK (id = 0), -- enforce single-row config
	cmdchars TEXT NOT NULL,

	nick TEXT NOT NULL,
	user TEXT NOT NULL,
	real TEXT NOT NULL,

	server TEXT NOT NULL,
	port INTEGER NOT NULL,
	ssl BOOL NOT NULL
);
INSERT INTO config (id, cmdchars, nick, user, real, server, port, ssl) VALUES (0, '.', 'testbot', 'testbot', 'testbot', 'irc.sorcery.net', 6667, false);

CREATE TABLE channels (
	channel TEXT NOT NULL PRIMARY KEY
);
INSERT INTO channels VALUES ('#bot32-test');
