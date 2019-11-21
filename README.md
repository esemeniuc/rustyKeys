# cpsc311Project
Main project for CPSC 311

Benchmark
```bash
redis-benchmark -n 10000 eval 'redis.call("SETNX __rand_int__ __rand_int__"); redis.call("INCR __rand_int__); redis.call("GET __rand_int__")'
```
