#! /bin/bash

tag=$1
repo_url=$2
branch=$3

[[ ! -e ./data/git ]] && mkdir data/git
cd data/git
if [[ ! -e "./$tag" ]]; then
	git clone "https://$repo_url" "$tag" >&2
	cd "$tag"
else
	cd "$tag"
	git fetch origin >&2
fi

git rev-parse "origin/$branch"
