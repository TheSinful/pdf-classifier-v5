i had the idea to pre-initialize memory for known sized objects like Vecs on construction
for the obvious speed improvements

but, this shouldn't extend into smaller vecs like constraints
Classifier::soft_constraints Classifier::hard_constraints are (as of this moment) 20 bytes (not counting any padding Rust may add)

that is so miniscule that it isn't required

BUT for something like classified_results, where the wrapper Vec (which will be converted from a HashMap to a Vec) will always have a set sizeof(total_pages) 

although this is trivial, and practically non-important i just wanted to paste my thoughts out 