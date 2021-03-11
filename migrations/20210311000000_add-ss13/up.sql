CREATE TABLE ss13_servers (
	id TEXT NOT NULL PRIMARY KEY,
	addr TEXT NOT NULL
);

CREATE TABLE ss13_server_names (
	id TEXT NOT NULL,
	name TEXT NOT NULL PRIMARY KEY,

	CONSTRAINT fk_id FOREIGN KEY (id) REFERENCES ss13_servers(id)
);

CREATE TABLE ss13_server_channels (
	id TEXT NOT NULL,
	channel TEXT PRIMARY KEY,

	CONSTRAINT fk_id FOREIGN KEY (id) REFERENCES ss13_servers(id)
);
