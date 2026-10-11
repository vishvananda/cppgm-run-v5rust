// N3485 15p2, 6.6p2: a goto to a label inside a function-try-block's compound
// statement or handler stays inside the try block; leaving a scope by the goto
// destroys the objects declared in it, and an exception after the jump reaches
// the function-try-block's handler: free functions, handlers, nested try
// blocks, constructors and destructors
