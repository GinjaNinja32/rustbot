#! /bin/bash

format_out() {
	if [[ "$multiline" == "true" ]]; then
		cat
	else
		awk 1 ORS=';  '
	fi
}

if [[ "$secure" == "true" ]]; then
	timeout="$HOME/timeout --just-kill --no-info-on-success --detect-hangups -h 600 -t 600"
else
	timeout="$HOME/timeout --just-kill --no-info-on-success --detect-hangups -h 10 -t 10 -m 102400"
fi

file=$1

if [[ $2 == "run" ]]; then
	if [[ "$secure" == "true" ]]; then
		{ DreamDaemon "$file.dmb" -invisible -safe; } 2>&1 | tail -n +4 | head -c 512
	else
		{ DreamDaemon "$file.dmb" -invisible -ultrasafe; } 2>&1 | tail -n +4 | head -c 512
	fi
	exit 0
fi

if [[ ! -e external/byondsetup ]]; then
	echo "command requires setup, see external/README.md"
	exit 0
fi
. external/byondsetup
cd dm/

output=$($timeout DreamMaker "$file.dme" 2>&1)
return=$?

if [[ $return != 0 ]]; then
	echo "$output" | tail -n +3 | format_out
else
	if [[ "$secure" != "true" && -e $file.rsc ]]; then
		echo "You attempted to use a resource file; this is blocked for security reasons."
	else
		$timeout "../$0" "$file" run | format_out
	fi
fi

[[ -e "$file.dme" ]] && rm "$file".*
