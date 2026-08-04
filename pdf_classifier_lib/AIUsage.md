This part of the project comes about following a realization that there needs to be a genuine
library for the C++ "middleware" of the project.
Sadly, I am not a C++ gigachad who can write immaculate code within a short period
Therefore, I am forced to use AI to handle the following:

    - Testing concepts with how they may be implemented in this language. Since Rust differs pretty heavily to that of C++
      my ideas may work in a Rust environment, but they do not account for the memory unsafety of C++ therefore AI acts
      as the 'rubber-ducky' to test each idea. Note that this is to test an existing concept, not conceptualize
      a solution to a problem which may be incorrect due to the naive tendency of AI. Rather, I may prove idea A,
      which Claude pushes back on, and proposes idea B, which I push back on and suggest idea C.
    - Act as a teacher for written code or implementing concepts that are entirely foreign to me
      generally, by the means of syntax. For instance, templating isn't too crazy at a basic level,
      but constraining, using `[[nodiscard]]` where it's appropriete, etc -are full of syntax
      I do not understand. Therefore, AI acts as a teacher to correct mistakes made and keep this
      somewhat timely.

Although I pride this project in not being slop, this is the best use of the tool in this case.
