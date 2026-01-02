

rather than storing a bunch of static weights (or dynamic ones if i go down the route of importing them from schema definitions via the python side) 

instead i've decided to use "constraints" which can be defined as hard or soft 
hard constraints are mainly structural constriants, i.e chapter cant be within a datatable
while soft constraints are things that have room for possibility and for rewards

so instead of something like: 

```rs
REWARD_first_child: f32 // maybe like 2.0 
```

we define a soft constraint 
```rs

struct FirstChild; 

impl SoftConstraint for FirstChild {
    // impl add by 2.0 with validation here
}
```

this abstracts constraint checking away from the main classifier, 
plus with logging in the future allows for easy tracing without bloated files
these are statically dispatched for efficency 

i'll likely give these priority systems to further improve performance
maybe something like: 


```rs

enum ConstraintPriority {
    CRITICAL, 
    // idk more things 
}

trait SoftConstraint {
    const PRIORITY: ConstraintPriority; 
    // impl
}
```

but hard constraints will always hold priority and just go down the list, 
maybe not actually since some will be more "demanding" than others
i could maybe dynamically sort them? 
too much work for now 
