PRAGMA foreign_keys = 1;

-------------------- GLOBAL --------------------

-- CONFIGS
CREATE TABLE configs (
	id TEXT NOT NULL PRIMARY KEY,
	type TEXT NOT NULL CHECK (type = 'irc' OR type = 'dis')
);
INSERT INTO configs VALUES ('irc', 'irc');
INSERT INTO configs VALUES ('discord', 'dis');

-- MODULES
CREATE TABLE modules (
	name TEXT NOT NULL PRIMARY KEY,
	enabled BOOL NOT NULL
);
INSERT INTO modules (name, enabled) VALUES ('core', true);

-- MODULES
CREATE TABLE enabled_modules (
	config_id TEXT NOT NULL,
	name TEXT NOT NULL,
	PRIMARY KEY (config_id, name),
	CONSTRAINT fk_config FOREIGN KEY (config_id) REFERENCES configs(id),
	CONSTRAINT fk_name FOREIGN KEY (name) REFERENCES modules(name)
);
INSERT INTO enabled_modules VALUES ('irc', 'core');
INSERT INTO enabled_modules VALUES ('discord', 'core');

-------------------- IRC --------------------

-- CONFIGS
CREATE TABLE irc_configs (
	id TEXT NOT NULL PRIMARY KEY,

	cmdchars TEXT NOT NULL,

	nick TEXT NOT NULL,
	user TEXT NOT NULL,
	real TEXT NOT NULL,

	server TEXT NOT NULL,
	port INTEGER NOT NULL,
	ssl BOOL NOT NULL,

	CONSTRAINT fk_id FOREIGN KEY (id) REFERENCES configs(id)
);
INSERT INTO irc_configs (id, cmdchars, nick, user, real, server, port, ssl) VALUES ('irc', '.', 'testbot', 'testbot', 'testbot', 'irc.sorcery.net', 6667, false);

-- Triggers to ensure irc_configs entries are only added for irc-type configs
CREATE TRIGGER irc_configs_type_insert BEFORE INSERT ON irc_configs BEGIN
	SELECT CASE
	WHEN (NOT EXISTS (SELECT type FROM configs WHERE id = NEW.id AND type = 'irc'))
	THEN RAISE(ABORT, 'Attempted to add an irc_configs entry for a config that does not exist or is not of type "irc"!')
	END;
END;
CREATE TRIGGER irc_configs_type_update BEFORE UPDATE OF id ON irc_configs FOR EACH ROW BEGIN
	SELECT CASE
	WHEN (NOT EXISTS (SELECT type FROM configs WHERE id = NEW.id AND type = 'irc'))
	THEN RAISE(ABORT, 'Attempted to add an irc_configs entry for a config that does not exist or is not of type "irc"!')
	END;
END;

-- PERMISSIONS
CREATE TABLE irc_permissions (
	config_id TEXT NOT NULL,
	nick TEXT NOT NULL,
	user TEXT NOT NULL,
	host TEXT NOT NULL,
	flags INTEGER NOT NULL,
	PRIMARY KEY (config_id, nick, user, host),
	CONSTRAINT fk_config FOREIGN KEY (config_id) REFERENCES irc_configs(id)
);
INSERT INTO irc_permissions VALUES ('irc', 'GinjaNinja32', 'nyx', 'gn32.uk', 15);

-- CHANNELS
CREATE TABLE irc_channels (
	config_id TEXT NOT NULL,
	channel TEXT NOT NULL,
	PRIMARY KEY (config_id, channel),
	CONSTRAINT fk_config FOREIGN KEY (config_id) REFERENCES irc_configs(id)
);
INSERT INTO irc_channels VALUES ('irc', '#bot32-test');


-------------------- DISCORD --------------------

-- CONFIGS
CREATE TABLE dis_configs (
	id TEXT NOT NULL PRIMARY KEY CHECK (id = 'discord'), -- discord config is a singleton currently
	cmdchars TEXT NOT NULL,
	bot_token TEXT NOT NULL
);
INSERT INTO dis_configs VALUES ('discord', '.', 'YOUR-BOT-TOKEN-HERE');

-- Triggers to ensure dis_configs entries are only added for dis-type configs
CREATE TRIGGER dis_configs_type_insert BEFORE INSERT ON dis_configs BEGIN
	SELECT CASE
	WHEN (NOT EXISTS (SELECT type FROM configs WHERE id = NEW.id AND type = 'dis'))
	THEN RAISE(ABORT, 'Attempted to add an dis_configs entry for a config that does not exist or is not of type "dis"!')
	END;
END;
CREATE TRIGGER dis_configs_type_update BEFORE UPDATE OF id ON dis_configs FOR EACH ROW BEGIN
	SELECT CASE
	WHEN (NOT EXISTS (SELECT type FROM configs WHERE id = NEW.id AND type = 'dis'))
	THEN RAISE(ABORT, 'Attempted to add an dis_configs entry for a config that does not exist or is not of type "dis"!')
	END;
END;

-- PERMISSIONS
CREATE TABLE dis_permissions (
	config_id TEXT NOT NULL,
	user_id INTEGER NOT NULL,
	flags INTEGER NOT NULL,
	PRIMARY KEY (config_id, user_id),
	CONSTRAINT fk_config FOREIGN KEY (config_id) REFERENCES dis_configs(id)
);
INSERT INTO dis_permissions VALUES ('discord', 169859930382270465, 15);
