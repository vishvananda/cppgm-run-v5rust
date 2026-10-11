// N3485 12.2p3, 12.2p5, 15.2p1: a temporary bound to a reference persists with
// it, while a temporary its initializer made is destroyed at the end of the
// full-expression; when an exception later leaves the block (a try block, a
// handler, a function-try-block, a loop), each is destroyed once: const and
// rvalue references, a reference member of an aggregate, two references and
// nested blocks
