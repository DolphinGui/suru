#!/usr/bin/sh

set -ex

sed -e 's/#REPLACEME#/2/g' $1 > $2
