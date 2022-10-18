#!/bin/bash

set -e

database_name="bagextract"
data="data" # or /data 
host="127.0.0.1" # or "$host"
export PGPASSWORD=$POSTGRES_PASSWORD

mkdir -p $data

## download bag
if [ ! -f $data/lvbag-extract-nl.zip ]; then
    # wget -q -O $data/lvbag-extract-nl.zip http://geodata.nationaalgeoregister.nl/inspireadressen/extract/lvbag-extract-nl.zip &
    curl -L -o $data/lvbag-extract-nl.zip https://service.pdok.nl/kadaster/adressen/atom/v1_0/downloads/lvbag-extract-nl.zip &
fi

# create db
psql -h $host -U tgbag postgres -c "DROP DATABASE IF EXISTS $database_name"
createdb -h $host -U tgbag $database_name
psql -h $host -U tgbag $database_name -c 'CREATE EXTENSION IF NOT EXISTS postgis'
psql -h $host -U tgbag $database_name -c 'CREATE EXTENSION IF NOT EXISTS postgis_topology'

psql -h $host -U tgbag $database_name < before.sql



cargo build --release &

# We use `&` to spawn child processes (which run in parallel). Now wait until all have completed
wait

num_name=`unzip -Z1 data/lvbag-extract-nl.zip | grep "NUM"`
vbo_name=`unzip -Z1 data/lvbag-extract-nl.zip | grep "VBO"`

# unzip -j $data/lvbag-extract-nl.zip $num_name $vbo_name -d $data

# mv $data/$num_name data/num.zip
# mv $data/$vbo_name data/vbo.zip

cargo run --release generate --source $data --user tgbag --password tgbag --host "$host" --dbname $database_name

psql -h $host -U tgbag $database_name < after.sql

# rm data/num.zip
# rm data/vbo.zip

# remove bag zip
# rm $data/lvbag-extract-nl.zip
