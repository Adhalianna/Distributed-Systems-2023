# WARNING!

Previously submitted solution might have contained an error resulting from
failure in using git to keep version between machines synchronized.
If the following error was observed:
```
thread 'main' panicked at 'failed to deserialize: Error("missing field `duration`", line: 6, column: 71)', task3/src/main.rs:381:39
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```
it should be now resolved. 

# Program input

The program requires a path to a json file as an argument. The examples can be
found in the [`inputs/task3`](../inputs/task3) directory. Following those 
examples new inputs can be created.

# Implementation
The algorithm is implemented by the `SystemNode` struct, in its `impl` block
the `iterate` function can be found which is the entrypoint to the
implementation on each iteration of a simulation. The tree datastructure is
implemented by the `Node` struct which uses internally reference counting
(smart pointer) to assure that all the nodes live long enough and no longer
than needed (borrow checker constraints). 

The `iterate` function starts with the node executing its tasks and as a result
producing messages to signal what it needs and whether it can give the token
away. 

# Examples

##### Example 1
[`inputs/task3/example1.json`](../inputs/task3/example1.json)
Program Output:
```
Reading from file inputs/task3/example1.json
ITERATION 2
--- Node B executes the idle task
--- Node "B" queue: []
--- Node A executes the idle task
--- Node "A" queue: []
ITERATION 3
--- Node B executes the idle task
--- Node "B" queue: []
--- Node A executes the idle task
--- Node "A" queue: []
ITERATION 4
--- Node B executes the idle task
--- Node "B" queue: []
--- Node A executes the idle task
--- Node "A" queue: []
ITERATION 5
--- Node B executes the idle task
--- Node "B" queue: []
--- Node A executes the idle task
--- Node "A" queue: []
ITERATION 6
--- Node B executes the idle task
--- Node "B" queue: []
--- Node A executes the idle task
--- Node "A" queue: []
ITERATION 7
--- Node B produced a request
--- Node "B" queue: ["B"]
--- Node A executes in the critical section
--- Node "A" queue: ["B"]
ITERATION 8
--- Node "B" queue: ["B"]
--- Node A executes in the critical section
--- Node "A" queue: ["B"]
ITERATION 9
--- Node "B" queue: ["B"]
--- Node A executes in the critical section
--- Node "A" queue: ["B"]
ITERATION 10
--- Node "B" queue: ["B"]
--- Node A executes in the critical section
--- Node "A" queue: ["B"]
ITERATION 11
--- Node "B" queue: ["B"]
--- Node A executes in the critical section
--- Node A relieves a token
--- Node "A" queue: []
ITERATION 12
--- Node B executes in the critical section
--- Node "B" queue: []
--- Node "A" queue: []
ITERATION 13
--- Node B executes in the critical section
--- Node "B" queue: []
--- Node "A" queue: []
ITERATION 14
--- Node B executes in the critical section
--- Node "B" queue: []
--- Node "A" queue: []
ITERATION 15
--- Node B executes in the critical section
--- Node "B" queue: []
--- Node "A" queue: []
ITERATION 16
--- Node B executes in the critical section
--- Node B relieves a token
--- Node "B" queue: []
--- Node "A" queue: []
FINISHED
```

##### Example 2
[`inputs/task3/example2.json`](../inputs/task3/example2.json)


##### Example 3
[`inputs/task3/example3.json`](../inputs/task3/example3.json)