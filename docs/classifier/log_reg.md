soft constraints themselves affect our dynamic "z" value 
so, soft constrains should have a budget that all of them add to 
and where instead they are tweaked accordingly 

since PDF documents are generally all the same (assuming they have a structure) 
these are a "soft" source of truth 

generally, our total constraint voice including to be added dynamic weights (i.e how often we see said class) 

should not be greater than 3.0 
z <= ±3.0 

that's our "major" budget, while we have "minor" budgets for specific weightings
like structural constraints, or dynamic weights, etc

but z <= ±3.0 is not a hard truth, we instead softcap z to ±3.0 

now the question is, what is our budget? 

how much do structural constraints get? how much do dynamic weights get? 
we have a "cap" which makes this easier 


currently, i'm thinking a budget of ±1.5 for structural constraints and ±1.0 for dynamic weights with the remaining ±0.5 being a buffer to the soft cap 

