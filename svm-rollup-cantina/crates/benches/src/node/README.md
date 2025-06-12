<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
**Table of Contents**  *generated with [DocToc](https://github.com/thlorenz/doctoc)*

- [Native Benchmarks](#native-benchmarks)
  - [Methodology](#methodology)
- [Makefile](#makefile)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Native Benchmarks
Native benchmarks refer to the performance of the rollup SDK in native mode - this does not involve proving
## Methodology
* We use the Bank module's Transfer call as the main transaction for running this benchmark. So what we're measuring is the number of value transfers that can be done per second. 
* We do not connect to the DA layer since that will be the bottleneck if we do. We pre-populate 20 blocks (configurable via env var BLOCKS) with 1 blob each containing 100 transactions each (configurable via env var TXNS_PER_BLOCK). 
* The first block contains a "CreateToken" and "Mint" transactions.
* All token transfers are initiated from the created token's mint address

```
* **rollup_coarse_measure.rs**
  * This script uses coarse grained timers (with std::time) to measure the time taken to process all the pre-generated blocks.
  * We can control the number of blocks and transactions per block with environment variables
  * There are timers around the main loop for a total measurement, as well as timers around key functions
    * begin_slot
    * apply_blob
    * end_slot
  * The script uses rust lib prettytable-rs to format the output in a readable way
  * Optionally, the script also allows generating prometheus metrics (histogram), so they can be aggregated by other tools.
```
+--------------------+--------------------+
| Blocks             | 20                 |
+--------------------+--------------------+
| Txns per Block     | 100                |
+--------------------+--------------------+
| Total              | 292.819598958s     |
+--------------------+--------------------+
| Begin slot         | 39.414µs           |
+--------------------+--------------------+
| End slot           | 243.091403746s     |
+--------------------+--------------------+
| Apply Blob         | 46.639351922s      |
+--------------------+--------------------+
| Txns per sec (TPS) | 3424.6575342465753 |
+--------------------+--------------------+
```

# Makefile
We abstract having to manually run the benchmarks by using a Makefile for the common benchmarks we want to run

The Makefile is located in the demo-rollup/benches folder and supports the following commands
* **make basic** - supports the coarse grained timers (getting the TPS) using rollup_coarse_measure.rs
* **make flamegraph** - runs `cargo flamegraph`. On mac this requires `sudo` permissions. The script ensures some cleanup and to err on the side of caution, it deletes the `sovereign/target` folder since new artifacts can be owned by root

The Makefile supports setting number of blocks and transactions per block using BLOCKS and TXNS_PER_BLOCK env vars. Defaults are 100 blocks and 10,000 transactions per block when using the Makefile

![Flamegraph](flamegraph_sample.svg)
