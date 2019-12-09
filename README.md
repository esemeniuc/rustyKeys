# rustyKeys

A Rust multithreaded implementation of Redis.

![RustyKeys Poster](https://raw.githubusercontent.com/esemeniuc/rustyKeys/media/poster.gif)
Benchmark
```bash
#GET and SET test
#sends: 
#tokens["SET", "key:000000000007", "xxx"]
#tokens["GET", "key:000000000032"]

redis-benchmark -n 100000 -r 50 -t GET,SET

#SETNX
#sends: tokens["SETNX", "key:000000000026", "DUMB_VAL"]
redis-benchmark -n 2 -r 50 SETNX key:__rand_int__ DUMB_VAL
```
