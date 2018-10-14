#!/bin/sh
ROOT=`dirname $0`
PROC=${ROOT}/target/release/mesi
count=`ps -ef | grep $PROC |grep -v grep | wc -l`
#echo "$count"
if [ $count = 0 ]; then
	cd $ROOT
    cargo build --release
	$PROC
else
	echo "already run"
fi
