#! /bin/bash

format_out() {
	if [[ "$multiline" == "true" ]]; then
		cat
	else
		awk 1 ORS=';  '
	fi
}

timeout="$HOME/timeout --just-kill --no-info-on-success --detect-hangups -h 10 -t 10 -m 102400"

file=$1

if [[ $2 == "run" ]]; then
	DreamDaemon "$file.dmb" -invisible -ultrasafe 2>&1 | tail -n +4 | head -c 512
	exit 0
fi

cd dm/

export BYOND_SYSTEM=/home/nyx/byond/lin/use
export PATH=$BYOND_SYSTEM/bin:$PATH
export LD_LIBRARY_PATH=$BYOND_SYSTEM/bin:$LD_LIBRARY_PATH

output=$($timeout DreamMaker "$file.dme" 2>&1)
return=$?

if [[ $return != 0 ]]; then
	echo "$output" | tail -n +3 | format_out
else
	if [[ -e $file.rsc ]]; then
		echo "You attempted to use a resource file; this is blocked for security reasons."
	else
		$timeout "../$0" "$file" run | format_out
	fi
fi

rm "$file".*
