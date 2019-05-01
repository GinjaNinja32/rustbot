#! /bin/bash

cd "$(dirname "$(readlink -f "$0")")"

if [[ $# != 1 ]]; then
	echo "usage: $0 migration-name"
	exit 1
fi

name="$1"

if ! [[ "$name" =~ ^[a-z0-9-]+$ ]]; then
	echo "migration names should be lowercase alphanumeric and dashes only"
	exit 2
fi

name="$(date +%Y%m%d000000)_$name"

mkdir "$name"
touch "$name/up.sql" "$name/down.sql"

if [[ "${EDITOR:-vim}" == vim ]]; then
	vim -o "$name/up.sql" "$name/down.sql"
else
	"${EDITOR:-vim}" "$name/up.sql" "$name/down.sql"
fi
