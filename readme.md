# BB_Challenge Library

## About

This started as a fun project to better understand the [[Busy Beaver Challenge] [https://bbchallenge.org/story]].

As the problem requires billions of calculations, I wrote this in Rust and chose a data structure which supports very performant calculations.

### Highlights

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

### Limitations

* Symbols: The data structure is very specific to BB problems with only 2 symbols.
* States: MAX_STATES: This const is defaulted to 5 for the BB_Challenge. It runs fine with n_states between 1 and 5.
6 and 7 will generally work, but will hit size and runtime issues. It can currently be set to a maximum of 7 states
(which is the limit of u64 which is used generally).The ids for n_states 8 and more will exceed
the u64 number range and thus are not permitted. You can lift this restriction and see what happens. Number overflow does not
create an error in release mode and is hard to detect.

* Tape Size: unknown: The tape is a vec of u32 (using bits, so one u32 represents 32 cells) which grows dynamically.

What the library can do:
*

Treat this as an **Alpha version**.
This means for instance:

* data structures may be altered
* function parameters may be altered
* function names may be altered
* enums may be altered
* Code is unfinished and may not work in all cases.
* Code is not reviewed or tested extensively.

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
For each transition, this results in 2 (symbols) *2 (directions)* 5 (states) + 1 (undefined) = 21 possibilities.  
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

## Terms used

This library is mostly designed for the Busy Beaver Game initiated by T. Rado.

* Turing Machine: A table of transitions for different states and symbols which could be called the program code.
* Binary Turing Machine: A Turing Machine with only the symbols 0 and 1.
* Step: A step is the execution of one transition of a Turing Machine. This means if the first transition is hold,
the machine will have one step. Rado uses the term 'shift', but also writes a symbol and shifts in the hold transition.
* Symbol: The symbol to be written, on a binary machine this is only 0 or 1. Rado used 'overwrite by' to illustrate
the fact, that the current tape cell is overwritten.
* Direction: Direction the head moves after the symbol is written. L for left, R for right, - for undefined in case of halt step.
* State: The state of the machine is an indexed number to allow different line access for the transitions. Rado used the term card.
The state is always written with a letter starting with A. The letter Z or dash '-' are used to indicate halt, where Z is used if
the last step writes a last symbol and then halts, whereas - is used if no symbol is written in the last step. The BB Challenge only
uses --- as last transition as it reduces the number of machines without changing the step count.
* Halt: The stop command of the machine.
* Tape: An infinite long tape comprised of cells (Rado: square) holding the symbols written.
* Cell: One field, square or storage space for one written symbol on the tape.
* Transition: A table cell of a Turing Machine. It contains the Symbol written, the direction to shift and the next state, e.g. 1RB.

## Machine Properties

A machine has a number of properties in the case it halts:

* n: number of states
* Steps: transitions used before halt
* Number of Ones: Number of ones written on the tape (Score in Rado terms)
* Sigma: ?
* tape length: Used tape cells. Defined by the extreme head positions in both directions.

## Features

* bb_debug: This will output detailed information on the steps in the Terminal.

## Enumerator

The enumerator in this library does not use the TNF tree usually used. When I started programming, I was
not aware of that algorithm, but now I think my algorithm may be faster. \
The beauty of the TNF algorithm is that is eliminates whole tree sections. But this comes with a price. Handling
trees is much more complex than handling a table where mostly one field needs to change only. A tree requires predecessor
and successor handling which hardly can be done on stack memory. It also is difficult to parallelize.

My algorithm basically creates all machines (limited to A0 0RB and 1RB) and then decides quickly, if it is
even relevant for the deciders. Going backward, this also allows to cut tree sections (not yet implemented), but
even without the tree elimination it is very fast as most work can be done on the stack and parallelization is easy.
Creating all 16,679,880,978,201 machines for BB5 and filtering those for the deciders takes about 8 hours on a single CPU.
Splitting this means less than one hour calculation time on a halfway modern computer.

## Reverse enumeration

Enumeration order:

|   |  0 | 1 |
| - | -- | - |
| A | 10 | 9 |
| B |  8 | 7 |
| C |  6 | 5 |
| D |  4 | 3 |
| E |  2 | 1 |

Enumeration begins with field E1. If the pre-decider now finds this combination is eliminated it also can cut the tree, \
e.g. 0RB---\_0LA0RB\_... always encounters a 0 in each new step (only A0 and B0 are used). Since
B0 is the last used field, all other fields can be reset to start and B0 increased by one. Similar logic applies if higher
fields are facing the same issue.

But there is no need to check all this.

Field A0: Always is the 1st field. Only possible transitions are 0RB or 1RB.\
Others are excluded for immediate Halt, NonHalt in case of state A, L R similarity and state similarity. \
Field B0: Always is the 2nd field. Only possible transitions are 0LB, 1LA, 1LB:

* ---: No, halt in step 2, cannot be most steps.
* 0LA: \
  If AO is 0RB then this is Non-Halt as only 0 is written. Can be checked easily by just checking on this combination. \
  If A0 is 1RB then 10 is written on the tape repeatedly in the same place. Non-Halt.
* 0LB: \
  If AO is 0RB then this is Non-Halt as only 0 is written. Can be checked easily by just checking on this combination. \
  If A0 is 1RB then possible.
* 0RA: Non-Halt, always moves right
* 0RB: Non-Halt, always moves right
* 1LA: possible
* 1LB: possible
* 1RA: Non-Halt, goes always right
* 1RB: Non-Halt, goes always right
* 0LC: possible
* 0RC: possible
* 1LC: possible
* 1RC: possible
* xxD, xxE: not possible for state symmetry

## Statistics

Of all possible machines less than 0,5% are relevant for deciders, all others do not meet the criteria.

For example BB(2,4): \

|                       | Count         |
| --------------------- | ------------- |
| Total machines        | 6,975,757,441 |
| Total machines        | 6,975,757,441 |
| Relevant for deciders |    30,199,552 (0,43%) |
| Decided Halts         |    10,758,178 |
| Decided Non-Halt      |    19,439,058 |
| - Cycler              |    19,378,244 |
| - Bouncer             |        60,814 |
| Undecided             |         2,316 |

Of the 30,199,552 machines a whopping 30,136,422 are decided by a cycler with a low step
limit as it decides Halt and Cycler. \
Notably, 10,752,250 (99.9945%) of the halting machines are decided in the first 25 steps. \
Same goes for the Cycler, where 99,4% are detected in the first 25 steps, 99,96% after 50 steps. \
This also holds true for larger machines with 5 or more states and only <0,000001% are running longer than 100 steps.

This shows the importance of having a fast cycler capable of running a few steps only. It can be executed directly
in the enumerator, eliminating the need to pass the machines into heap memory to be be able to run deciders on them.
