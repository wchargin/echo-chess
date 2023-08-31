# echo-chess

I enjoyed reading <https://samiramly.com/chess>, and was following along
happily through the cool puzzles and engaging writing, until I got to the point
about adding ML systems. I thought that this should *probably* be an easy
problem with a pretty tiny state space, and at worst that an off-the-shelf TSP
or SAT solver would be able to do a good job at it.

So, I nerd-sniped myself into writing a quick solver. This is my literal first
attempt (worked first try!), using naive breadth-first search, with no attempts
at optimization. I haven't optimized it at all, but I did design it with
performance in mind; hot types are kept in machine words (`u32`s), and
available moves are computed using SIMD-in-a-register style arithmetic (see the
`impl Stepper for _` blocks). I think that trick is pretty cool.

The blog post gives an example of a puzzle that "*feels* unsolvable the first
dozen times you try it, but that does actually have known solutions". That
puzzle took me about ten minutes to solve manually. This solver finishes it in
about **24 microseconds** on my laptop. Granted, it only goes through about 220
states, but that speaks to my point about the state space being small.

I think an upper bound on the possible size of the state space for a puzzle
with `n` pieces is something like `n * 2^(n-1)`. (A `PuzzleState` is described by
which of the `n` pieces is currently active, and which of the other pieces
still need to be captured.) So, even with 16 pieces in the very worst case,
this is something like a million states. And I wrote this in *(checks notes)*
two and a half hours. I think it's worth at least giving a solver like this a
shot, maybe even tuning it a little if it's too slow, before turning to all the
complexities of machine learning!

(Code quality: the engine core is pretty well written; the testing and main
shell and parsing are absolutely shoddy because I don't really care.
¯\\\_(ツ)\_/¯ )

## Running

Install Rust, then `cargo run --release`.

(A puzzle is hard-coded in "compound FEN" notation near the end of `fn main`.
You can substitute your own and re-compile/re-run if you want.)
