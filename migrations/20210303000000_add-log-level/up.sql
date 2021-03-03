CREATE TYPE log_level AS ENUM ('error', 'warn', 'info', 'debug', 'trace');

ALTER TABLE modules ADD COLUMN log_level log_level;
