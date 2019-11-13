-------------------- GLOBAL --------------------

-- CONFIGS
CREATE TABLE configs (
	id TEXT NOT NULL PRIMARY KEY,
	cmdchars TEXT NOT NULL,
);
INSERT INTO configs VALUES ('irc', '!');
INSERT INTO configs VALUES ('discord', '!');

-- MODULES
CREATE TABLE modules (
	name TEXT NOT NULL PRIMARY KEY,
	enabled BOOL NOT NULL
);
INSERT INTO modules (name, enabled) VALUES ('core', true);

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

-- PERMISSIONS
CREATE TABLE irc_permissions (
	config_id TEXT NOT NULL,
	nick TEXT NOT NULL,
	username TEXT NOT NULL,
	host TEXT NOT NULL,
	flags BIGINT NOT NULL,
	PRIMARY KEY (config_id, nick, username, host),
	CONSTRAINT fk_config FOREIGN KEY (config_id) REFERENCES configs(id)
);
INSERT INTO irc_permissions VALUES ('irc', 'GinjaNinja32', 'nyx', 'gn32.uk', 31);

-- CHANNELS
CREATE TABLE irc_channels (
	config_id TEXT NOT NULL,
	channel TEXT NOT NULL,
	PRIMARY KEY (config_id, channel),
	CONSTRAINT fk_config FOREIGN KEY (config_id) REFERENCES configs(id)
);
INSERT INTO irc_channels VALUES ('irc', '#bot32-test');


-------------------- DISCORD --------------------

-- PERMISSIONS
CREATE TABLE dis_permissions (
	config_id TEXT NOT NULL,
	user_id BIGINT NOT NULL,
	flags BIGINT NOT NULL,
	PRIMARY KEY (config_id, user_id),
	CONSTRAINT fk_config FOREIGN KEY (config_id) REFERENCES configs(id)
);
INSERT INTO dis_permissions VALUES ('discord', 169859930382270465, 31);
