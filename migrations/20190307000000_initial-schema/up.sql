PRAGMA foreign_keys = 1;

-- MODULES
CREATE TABLE modules (
	name TEXT NOT NULL PRIMARY KEY,
	enabled BOOL NOT NULL
);
INSERT INTO modules (name, enabled) VALUES ('core', true);

-- BEGIN IRC

-- CONFIG
CREATE TABLE irc_config (
	id TEXT NOT NULL PRIMARY KEY,

	cmdchars TEXT NOT NULL,

	nick TEXT NOT NULL,
	user TEXT NOT NULL,
	real TEXT NOT NULL,

	server TEXT NOT NULL,
	port INTEGER NOT NULL,
	ssl BOOL NOT NULL
);
INSERT INTO irc_config (id, cmdchars, nick, user, real, server, port, ssl) VALUES ('irc', '.', 'testbot', 'testbot', 'testbot', 'irc.sorcery.net', 6667, false);

-- PERMISSIONS
CREATE TABLE irc_permissions (
	config_id TEXT NOT NULL,
	nick TEXT NOT NULL,
	user TEXT NOT NULL,
	host TEXT NOT NULL,
	flags INTEGER NOT NULL,
	PRIMARY KEY (config_id, nick, user, host),
	CONSTRAINT fk_config FOREIGN KEY (config_id) REFERENCES irc_config(id)
);
INSERT INTO irc_permissions VALUES ('irc', 'GinjaNinja32', 'nyx', 'gn32.uk', 1);

-- CHANNELS
CREATE TABLE irc_channels (
	config_id TEXT NOT NULL,
	channel TEXT NOT NULL,
	PRIMARY KEY (config_id, channel),
	CONSTRAINT fk_config FOREIGN KEY (config_id) REFERENCES irc_config(id)
);
INSERT INTO irc_channels VALUES ('irc', '#bot32-test');

-- END IRC

-- BEGIN DISCORD

-- CONFIG
CREATE TABLE dis_config (
	id INTEGER NOT NULL CHECK (id = 0), -- discord config is a singleton
	cmdchars TEXT NOT NULL,
	bot_token TEXT NOT NULL
);
INSERT INTO dis_config VALUES (0, '.', 'YOUR-BOT-TOKEN-HERE');

-- PERMISSIONS
CREATE TABLE dis_permissions (
	user_id INTEGER NOT NULL,
	flags INTEGER NOT NULL,
	PRIMARY KEY (user_id)
);
INSERT INTO dis_permissions VALUES (169859930382270465, 1);
