CREATE TABLE mpg (
	mileage INT4 PRIMARY KEY,
	fill_litres FLOAT8 NOT NULL,
	fill_price FLOAT8 NOT NULL,
	result_price FLOAT8 -- NULL if tank not full
);
