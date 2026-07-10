This document is to justify the minimal usage of AI throughout this project, 
as this project exists as a "show-piece" for myself. Therefore I believe it is imperative to be transparent with the most intellectually degrading tool available; AI.

## Criterias

The following, are the criterias that I went through before deciding whether AI could complete a task: 
1. Is the task going to take a significant amount of time, while offering little "development"? 
    - For instance, "boilerplate" code.
2. Is the task intellectually viable? 
    - Will someone look at this part of the project and feel like I put minimal effort or rather decided to cheap out?
3. Is the task going to burn me out? 
    - Will repition of this task burn my passion for the project? 
4. (Special-case) If I wrote this solution already, is the task simply reintegrating that original code? 
    - In reference to object tests.

## Justification

Personally, I disagree with just throwing claude on agent mode and letting it just make a chunk of the project. I prefer the idea of using AI as the ultimate "rubber-ducky", and this was precisely what AI was used for, conceptual idea over code-generation. Furthermore, I know myself to where I know that repeatedly writing the same, boring code will burn me out of a project, some code is better than no-code. For the fourth point, I have already wrote an existing 'static' classifier for specifically the example document type. That project, was quite frankly horrific and no person would be able to understand it without having an aneurysm, therefore I chose to rewrite it when implementing its techniques for classification. Although the majority of said refactoring was done by myself, some aspects were not. For example, the tests made for each object type. The refactor restructured the visibility of many utility methods that had to be individually tested, and whatnot. Therefore, in this case AI became the best tool for the job, as there was no development nor growth through this task. 

## Specific Use-cases

1. Schema-tests within python front-end
    - Would've taken a good day or two, had minimal personal growth potential, and likely would've burned me out. 
2. Code-generation for repetitive traits (i.e Display)
    - Frankly, I was in a point of burn-out here, and should've done this myself, but nonetheless it happened. 
3. Tracing throughout core and front-end. 
    - I added tracing after developing, including within threading and whatnot. It became a monumental task to add individual trace steps, when AI could've done better formatting, better quality and consistency throughout traces. I definitely would've burned out if I had taken the task. 
4. Object-tests 
    - I explained this within justification.
5. Datatable visualization 
    - This was originally a C++ extension of the DataTable class in the original classifier, but as it would've polluted the datatable namespace, I decided to throw it onto Python as I required it for debugging purposes. 