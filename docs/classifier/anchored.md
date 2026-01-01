rather than forcing a complex approach for all document types
utilize an anchored approach for shallow object hierarchies

this came about after thinking deeper upon the defer approach used on the general system
and would be better for the example document

if objects look like:
chapter
| subchapter
| | diagram (pair)
| | table (pair)

rather than classifying deeply with constant deference, we can just search for subchapters specificalyl
attempt to classify every page as subchapter or chapter
each subchapter is the independent in this scenario
and simply infer diagram-table pairs within each subchapter

so
n(0..10)

n(0) chap # classified
n(5) sub # classified
n(10) sub # classified
n(0..4) [diagram, table, diagram, table] # inferred
n(6..9) [diagram, table, diagram, table] # inferred

n(0..10) therefore will only have i \* o classifications
where i is the total pages
and where o is the count of independent objects

c = io
c = 10 \* 2
or 20 classification cycles total

while n(0..4) and n(6..9) respectively will have zero classification after independents were classified
they will just call extract on their respective pattern
