CREATE TABLE mod_weather_config (
	id INTEGER NOT NULL CHECK (id = 0), -- singleton

	appid TEXT NOT NULL
);
INSERT INTO mod_weather_config VALUES (0, 'YOUR-APPID-HERE');
