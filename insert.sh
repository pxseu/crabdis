#!/bin/bash

# Script to insert 10 million keys into Redis using pipelining for efficiency.
# Keys will be in the format "key:N" where N is 1 to 10000000.
# Values will be a fixed long string of 10000 'a's for each key.
# Assumes redis-cli is installed and Redis is running on localhost:6379.
# Run with: ./insert_redis_long.sh
# WARNING: This will insert approximately 100 GB of data into Redis (10M keys * 10KB values, plus overhead).
# Ensure your Redis instance has sufficient memory and disk space. This operation may take a long time and could crash Redis if resources are insufficient.

# Generate the long value once outside the loop
long_val=$(printf '%*s' 10000 | tr ' ' 'a')
vl=${#long_val}  # Length is fixed at 10000

i=1
while [ $i -le 10000000 ]; do
  key="key:$i"
  kl=${#key}
  printf "*3\r\n\$3\r\nSET\r\n\$%d\r\n%s\r\n\$%d\r\n%s\r\n" "$kl" "$key" "$vl" "$long_val"
  i=$((i + 1))
done | redis-cli --pipe

echo "Insertion complete."
