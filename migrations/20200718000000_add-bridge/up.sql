CREATE TABLE mod_bridge (
	config_id TEXT NOT NULL,
	channel_id TEXT NOT NULL,
	bridge_key TEXT NOT NULL,
	PRIMARY KEY (config_id, channel_id)
);
