
define cargotoml
[package]
name = "PACKAGE"
version = "0.1.0"
authors = ["$(shell git config user.name) <$(shell git config user.email)>"]

[lib]
crate_type = ["dylib"]

[dependencies]
shared = { path = "../shared" }
endef
export cargotoml

define librs
extern crate shared;

use shared::prelude::*;

#[no_mangle]
pub fn get_meta() -> Meta {
    let mut meta = Meta::new();
    meta.command("foo", foo);
    meta
}

fn foo(ctx: &Context, args: &str) -> Result<()> {
	Ok(())
}
endef
export librs

mod_%:
	mkdir -p "$@/src"
	echo "$$cargotoml" | sed 's/PACKAGE/$@/g' > $@/Cargo.toml
	echo "$$librs" > $@/src/lib.rs


mkdir_data:
	mkdir data/

data: mkdir_data data/airports.csv

data/airports.csv:
	curl -s https://raw.githubusercontent.com/jpatokal/openflights/master/data/airports.dat \
		| cut -d, -f5-8 \
		> $@
