im gonna start using these "docs" to spit out my thoughts onto text,
mainly because i want to track them and be able to go back on them
but also so i can get a deeper understanding

they obviously won't have "professional" terminology nor good english
all i give a shit about is designing this the best way i can

so firstly,

classification != inference, inference happens prior to classification
and classification is directly tied to inference

inference is the "model" to weigh all constraints and deny some types
(types in this case refers to the pool of inferrable objects i.e chapter, subchapter, etc)

the idea is to hold inference sequentially which should be "cheap" but keeping computationally expensive parts away, away in the sense of off the same thread

lets say we have n pages where i is the current type being inferenced for n(i) page

in this case, we'll state that classification takes 10x longer than inference

so we'll batch into 10 pages at once (n <= 10)

well, we won't explicitly batch since we can't run 10 inferences at the same time, rather this case will only look at 10 pages

where:
n = 1: chapter
n = 2: subchapter
n = 3: diagram
n = 4: datatable
continue to 8
n = 9: subchapter
n = 10: diagram

as inference iterates over n pages it will have i types to infer over
with respect to context

schematically we define "chapter" as root
therefore, n = 1, i = chapter (by significant weight) let's say the max weight we can give
maybe we'll just ignore any other types since chapter has such a high weight
but that contradicts the fundamental idea of dynimicallity

n(num) = i 
n(2) = subchapter with high weighting due to only_valid_child
n(3) = diagram with high weighting due to new_pair and first_child
n(4) = datatable with high weighting due to end_pair
n(5) = diagram || subchapter
n(5) this is where dynamic weightings come into play, although in this case

we only iterate over 10 therefore don't have enough of a sample size to prioritize one over the other (which would be through average number of diagram-table pairs) in a genuine document
this would likely have been defined if we were half way through the document and probably would be pretty accurate

so, if that number was 4.3 (we would round) and n(5) = diagram safely
but in this case we should get identical structural weights then would test either one (lets say subchapter) if that fails, we safely know n(5) = diagram this would affect our average to 2.0 since n(6) will be inferenced as datatable

but what do we do if this was "breaking" 
lets step back to n(5) 
n(5) = subchapter || diagram 
BUT n(6) = diagram || datatable entirely dependent upon if n(5) is either of the two 
so, in a riskier model we would take subchapter (assuming no average diagram-table) 
but in a safer one we would take diagram, since if n(5) was actually subchapter we would end up breaking our correct inference

on failure, we wouldn't instantly throw out the inference of n(6) which in this case would likely be wrong, because of submission to dynamicality, rather we would store the classification result and re-run off further context of n(5) 

in this case, by the time we figure out these issues n(7..10) would have finished classification and we would repurpose those threads onto n(10..13) with a penalty onto 
non-breaking objects 

when i mean non-breaking i mean, if we aren't sure n(5) = subchapter, then we cant confirm n(13) = i since the document structure may shift

but if n(13) could equal something that isn't affected by the document structure shift like a new chapter then it would get a reward

the only issue I could see with this is that n(10..13) may all become chapters 

so rather, on a "breaking" object like n(5) = subchapter if n(5) != diagram (via classification) we treat it as a deferred block, 
we store all the next inferences upto an independent object (like a new subchapter) as hypotheses NOT inference nor will classification run within dependent objects (until we return) 
now lets say n(10) = subchapter
n(5..9) is a deferred block, n(10..n > 10) is apart of a new block 
we now come back to that deferred block to finish inference
n(5) = subchapter 
n(6) = diagram
n(7) = datatable 
n(8) = diagram
n(9) = datatable

with these hypotheses (which are just inferenced the same but relative to the actual result of n(5) when it was a breaking change) 

but wait, what if n(5) wasn't breaking, well then we discard the deferred block and continue as if nothing wrong happened

maybe the deferred block will hypothesize just off structural weights NOT dynamic ones and then once we find an independent we can re-inference which although slightly more expensive will be faster than two full inference runs

a defer block itself implies that start_page breaks the entire defer block, otherwise we wouldn't have deferred. 

defer happens if 
on breaking_posibility (non_breaking object or breaking object) 
classification fails as non_breaking object
therefore, the dependent objects are "poisioned" 
this is where we then simply hypothesize 

well, rather than hypothesizing we can weight independent objects, i.e subchapter higher (or rather dependent ones lower than independent ones), since the breaking ones are already most likely incorrect
in the example: 
n(5) = subchapter 
n(6) = diagram
n(7) = datatable 
n(8) = diagram
n(9) = datatable
if subchapter was the breaking object, the n(6..9) are effictively irrelvent to hypothesize as diagram-tables are dependent upon n(5) while n(5) is breaking
we only care about if n(6..9) contains an independent to end the defer 
although this would hypothetically "waste" cycles, well no it wouldn't because n(6..9) can be inferred safely since we found the two independents around it 

could that in itself be a different way of classification? looking for independents rather than classifying all and inferring based on adjascent structure? 