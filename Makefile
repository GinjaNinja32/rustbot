
define cargotoml
[package]
name = "PACKAGE"
version = "0.1.0"
authors = ["$(shell git config user.name) <$(shell git config user.email)>"]
edition = "2018"

[lib]
crate_type = ["dylib"]

[dependencies]
rustbot = { path = "../rustbot" }
endef
export cargotoml

define librs
use rustbot::prelude::*;

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd("foo", Command::new(foo));
}

fn foo(ctx: &dyn Context, args: &str) -> Result<()> {
	Ok(())
}
endef
export librs

mod_%:
	mkdir -p "$@/src"
	echo "$$cargotoml" | sed 's/PACKAGE/$@/g' > $@/Cargo.toml
	echo "$$librs" > $@/src/lib.rs


mkdir_data:
	mkdir -p data/

.PHONY: data
data: mkdir_data data/airports.csv

define AIRPORT_PROCESSING
import csv
import sys

for row in csv.reader(sys.stdin):
	print ",".join(row[4:8])
endef
export AIRPORT_PROCESSING

data/airports.csv:
	curl -s https://raw.githubusercontent.com/jpatokal/openflights/master/data/airports.dat \
		| python2 -c "$$AIRPORT_PROCESSING" \
		> $@
