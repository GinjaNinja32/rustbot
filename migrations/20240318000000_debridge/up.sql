CREATE TABLE mod_debridge (
	config_id TEXT NOT NULL REFERENCES configs (id),
	source_user TEXT NOT NULL,
	spec TEXT NOT NULL,

	PRIMARY KEY (config_id, source_user)
);
