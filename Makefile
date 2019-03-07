update:
	# Updates Cargo.toml to reflect module status
	truncate --size=0 Cargo.toml
	echo '[workspace]' >> Cargo.toml
	echo 'members = [' >> Cargo.toml
	echo '"rustbot",' >> Cargo.toml
	echo '"shared",' >> Cargo.toml
	find mod_* -mindepth 1 -maxdepth 1 -name Cargo.toml \
		| sed -e 's|^./||g' -e 's|/Cargo.toml$$||g' -e 's/.*/"\0",/' \
		| sort \
		>> Cargo.toml
	echo ']' >> Cargo.toml

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

use shared::types;

#[no_mangle]
pub fn get_meta() -> types::Meta {
    let mut meta = types::Meta::new();
    // meta.command("foo", foo);
    meta
}
endef
export librs

mod_%:
	mkdir -p "$@/src"
	echo "$$cargotoml" | sed 's/PACKAGE/$@/g' > $@/Cargo.toml
	echo "$$librs" > $@/src/lib.rs

