# Program input

The program requires a path to an input file (json) format as its first
argument. The format describes requests for resources issued by nodes in
a network. A special parameter `global_order` is used to make a node lock
on a specific resource before another requests for it. The exact order among
all requests is not important hence the examples use for that field only
values 0 and 1.

# Provided examples


##### Example 1
[`inputs/task5/example1.json`](../inputs/task5/example1.json)  
A simple deadlock example - three nodes are locking on their own resources
while asking each other for access to another node's resources.


##### Example 2
[`inputs/task5/example2.json`](../inputs/task5/example2.json)  
An example of a non-deadlock situation.



##### Example 3
[`inputs/task5/example3.json`](../inputs/task5/example3.json)  
Similar to example 1 but the chain of requests is longer and the transit rule 
can be observed.  
Program output:
```
Reading from file inputs/task5/example3.json
Node A starts with labels (pub: 1, priv 1)
Node B starts with labels (pub: 2, priv 2)
Node C starts with labels (pub: 3, priv 3)
Node D starts with labels (pub: 4, priv 4)
Node E starts with labels (pub: 5, priv 5)
Node F starts with labels (pub: 6, priv 6)
Node F sent request for resources to node F
Node E sent request for resources to node E
Node D sent request for resources to node D
Node C sent request for resources to node C
Node B sent request for resources to node B
Node A sent request for resources to node A
Node F sent request for resources to node A
Node F changed labels to (pub: 7, priv 7)    <-- Block rule
Node F received the transit message
Node E sent request for resources to node F
Node E changed labels to (pub: 8, priv 8)
Node E received the transit message
Node D sent request for resources to node E
Node D changed labels to (pub: 9, priv 9)
Node D received the transit message
Node C sent request for resources to node D
Node C changed labels to (pub: 10, priv 10)
Node C received the transit message
Node B sent request for resources to node C
Node B changed labels to (pub: 11, priv 11)
Node B received the transit message
Node A sent request for resources to node B
Node A changed labels to (pub: 12, priv 12)
Node F received the transit message
Node F changed labels to (pub: 11, priv 7)   <-- Transit rule
Node E received the transit message
Node E changed labels to (pub: 11, priv 8)
Node D received the transit message
Node D changed labels to (pub: 11, priv 9)
Node C received the transit message
Node C changed labels to (pub: 11, priv 10)
Node B received the transit message
Node B has detected a deadlock!
Scenario finished running.
```

##### Example 4
[`inputs/task5/example4.json`](../inputs/task5/example4.json)  
Another example of not a deadlock. Two nodes ask two other nodes for the same
requests.

# Implementation

The _main loop_ processes all the collected requests and uses them to drive the
scenario to completion. Each request starts a cascade of messages.