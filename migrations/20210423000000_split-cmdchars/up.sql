CREATE TABLE cmdchars (
	config_id TEXT NOT NULL,
	channel TEXT NOT NULL,
	cmdchars TEXT NOT NULL,

	PRIMARY KEY (config_id, channel)
);
INSERT INTO cmdchars (config_id, channel, cmdchars)
	SELECT id, '%', cmdchars FROM configs;

ALTER TABLE configs DROP COLUMN cmdchars;
