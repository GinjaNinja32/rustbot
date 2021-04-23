ALTER TABLE configs ADD COLUMN cmdchars TEXT NOT NULL DEFAULT '';

UPDATE configs SET cmdchars = cmdchars.cmdchars FROM cmdchars WHERE cmdchars.config_id = configs.id AND cmdchars.channel = '%';

ALTER TABLE configs ALTER COLUMN cmdchars DROP DEFAULT;

DROP TABLE cmdchars;
