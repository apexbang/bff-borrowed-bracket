#!/bin/bash
# usage: batch.sh first_seed last_seed parallel threads_each [extra args...]
first=$1; last=$2; par=$3; th=$4; shift 4
seq $first $last | xargs -P $par -I{} sh -c \
  "./target/release/soup --quiet --seed {} --threads $th --stop-after 1024 --out runs/s{} $* > runs/s{}.out 2>&1"
