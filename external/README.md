## External dependencies

This folder should contain symlinks or files describing how the bot should call external dependencies such as BYOND. None of these files are required for core bot functionality, but certain modules will require some or all of them to be present.

### byondsetup

This should be a symlink to or copy of the `byondsetup` script installed by BYOND, which sets the BYOND_SYSTEM environment variable and adds the required entries to PATH and LD_LIBRARY_PATH to call DreamMaker and DreamDaemon on the command line.

This file is sourced by `scripts/dm_compile_run.sh`, and is required for the `dm` module to function.

### paste

This should be a script that accepts input on stdin, stores it somewhere, and outputs the URL to access the stored data on stdout; for example, it could store data in a directory served by a webserver.
The filename the data is written to should _not_ be static; a good choice might be a hash of the input data.
