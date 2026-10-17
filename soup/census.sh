#!/bin/bash
# Replay a seed up to (just past) its transition with snapshots, then for each
# snapshot: census of real copiers, and copiers expected from composition alone.
# usage: census.sh seed last_epoch every threads
seed=$1; last=$2; every=$3; th=$4
out=runs/census_s$seed
./target/release/soup --quiet --seed $seed --epochs $last --snapshot-every $every \
  --threads $th --out $out > $out.out 2>&1
for f in $out/soup_0*.bin; do
  ep=$(basename $f .bin | sed 's/soup_0*//')
  c=$(./target/release/search --census $f --threads $th | head -1)
  l=$(./target/release/search --like $f --samples 2000000 --threads $th | head -1)
  echo "epoch $ep | $c | $l"
done > $out/census.txt
