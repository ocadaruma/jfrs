#!/bin/bash

set -eu

# A script to generate example jfr files to test the reader works.

DURATION_SECONDS=15
async_profiler_path=$ASYNC_PROFILER_PATH
output_path=$OUTPUT_PATH

cd "$(dirname $0)"

javac Example.java
java Example $OUTPUT_PATH &

app_pid=$!

echo "Started app in PID: $app_pid"

jcmd $app_pid JFR.start duration=${DURATION_SECONDS}s filename=$PWD/recording.jfr

sleep $DURATION_SECONDS
# sleep another 1 second to ensure recordings are stop
sleep 1

$async_profiler_path/profiler.sh -d $DURATION_SECONDS -e wall -t -f profiler-wall.jfr $app_pid
$async_profiler_path/profiler.sh -d $DURATION_SECONDS -e lock -t -f profiler-lock.jfr $app_pid
$async_profiler_path/profiler.sh -d $DURATION_SECONDS -e alloc -t -f profiler-alloc.jfr $app_pid
$async_profiler_path/profiler.sh -d $DURATION_SECONDS --chunksize 10k --chunktime 5s -e wall -t -f profiler-multichunk.jfr $app_pid

kill $app_pid
