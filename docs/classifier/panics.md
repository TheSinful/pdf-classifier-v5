within the classifier itself i believe it is trivial to just utilize panic! 

why? 

well the classifier itself (especially during inference) needs to be as fast as possible
if classification takes c amount of time, inference time takes i time and we have n threads

ni < c

we need to be done with a batch of classification before classification finishes incase we need to 
yield for a defer. 
