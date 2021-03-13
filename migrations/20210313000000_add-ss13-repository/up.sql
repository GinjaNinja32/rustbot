CREATE TABLE ss13_repositories (
	id TEXT NOT NULL PRIMARY KEY,
	branch TEXT NOT NULL,
	repo_url TEXT NOT NULL,

	CONSTRAINT fk_id FOREIGN KEY (id) REFERENCES ss13_servers(id)
);
