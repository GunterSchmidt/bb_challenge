# BB_Challenge Library

## About
This startet as a fun project to better understand the [[Busy Beaver Challenge] [https://bbchallenge.org/story]]. 

As the problem requires billions of calculations, I wrote this in Rust and chose a data structure which supports very performant calculations.

**Highlights**

Fast and flexible Generator:
* Generates all possible machines.
* Has a Pre-Decider which runs during generation and allows to eliminate around 99% of the machines for various reasons as these cannot be the longest running (max steps.). So only 1% of the possible machines need to be checked in detail.  
Example BB4: From the 6,975,757,441 machines only 54,588,416 (0.8%) are generated. Of these 23,746,060 hold and 30,769,060 are decided endless just from the Loop Decider. Only 73,296 require further decider logic.  
This allows to calculate BB4 in just a few seconds (on a normal 4 core notebook less than 20 seconds).
* Is returning the same ids for the machine, regardless if full generation or with pre-decider elimination.
* Produces batches of machines and can run in parallel threads.
* Can start at any batch (first id in batch is multiple of batch size) without generating all previous machines. 

Turing Machine:
* Can be created by generator, TM Standard Text Format or reads bb_challenge file (by id, range or \&\[ids\])
* Two formats available, generic format allows reading of TM with higher state and symbol limits. 2-Symbol format is fast but limited in size, see below.

**Limitations**
* Symbols: The data structure is very specific to BB problems with only 2 symbols. 
* States: MAX_STATES: This const is set to 5 for the BB_Challenge. It can currently be set to a maximum of 7 states (which is the limit of u64).
* Tape Size: unknown: The tape is a vec of u32 (using bits, so one u32 represents 32 cells) which grows dynamically.



What the library can do:
* 

Treat this as a late Alpha version.
application code is to test and run stuff in the [[bb_challenge library](https://github.com/GunterSchmidt/bb_challenge)].

It contains a bunch of test code, most of it can be disregarded, but may help to identify how the library is used.

Note: Code in test_run_deciders is deprecated and should not be used.

It might be helpful to have both in a workspace with a cargo.toml looking like this:

[workspace]  
resolver = "2"  
members = ["bb_challenge", "busy_beaver"]  
default-members = ["busy_beaver"]  

---

## Permutations

The general formula to calculate the possible machines is (4*s+1)^2*s (s = number of status).  
For each transition, this results in 2 (symbols) * 2 (directions) * 5 (states) + 1 (undefined) = 21 possibilites.  
In the transition table there are 5 (current state) * 2 (current symbol) = 10 fields, so to the power of 10.  

Number of machines for:  
BB=1: 25
BB=2: 6.561
BB=3: 4.826.809 (4.8 million)
BB=4: 6.975.757.441 (7 billion)
BB=5: 16.679.880.978.201 (16.7 trillion)
BB=6: 59.604.644.775.390.600 (59.6e15)
BB=7: 297.558.232.675.799.000.000 (257.6e18), Limit 64-Bit
BB=8: 1.977.985.201.462.560.000.000.000 (2e24)
BB=9: 16.890.053.810.563.300.000.000.000.000
BB=10: 180.167.782.956.421.000.000.000.000.000.000, Limit 128-Bit
