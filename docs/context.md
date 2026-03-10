


Context refers to the Context struct (src/context.rs) 
these notes are meant to provide some clarity as to how i plan to manage context, as context is practically synonymous with state. 

firstly, there are a few issues that come to mind with the original approach: 
    
    1. Size: in deference, how do we backtrack context to a state? especially when deference could happen "n" pages back
        where "n" refers to the number of threads allocated towards classification, since we could queue a batch of "n" classification tasks
        and since, they will roughly take similar amounts of time (assuming decent user classification code) 
        inherently, that whole batch will be fucked up if "n1" fails especially if no independent is within "n1".."n(max)"
        therefore, in such a scenario if we wanted to backtrack our state we would need an approach that doesn't force "n" clones of context 
        prior to spawning of a batch.
    2. Excessive allocations: same point as above, but re-iterated since context will become expensive with large pages
    3. Safety: in a linear approach, i believe it is risk prone to have to manage "n" states constantly, especially if those contexts are passed around
        throughout workers


So that is why when re-designing Context i created ContextUpdates, which in theory would allow a signficantly faster approach to just backtracking a single context
But that still is pretty slow, since you iterate over each update then revert backtracking the state, and atleast at the time of writing this, means losing state
from the remainder of inferenced classes in said batch. or in other words, if "n1" failed, we lose the result of "n9" that would eliminate one class from the pool.
and therefore increase the expense of inference and classification of "n9" when it is re-done. 
