// N3485 5.3.4p18, 5.3.4p20, 15.2p2: when the initialization of an object a
// new-expression creates throws, the matching deallocation function frees the
// storage; here the initializer allocates and deletes an array of class
// objects whose elements may throw (with no elements too): expression
// statements, initializers, nested new-expressions, try blocks, handlers, a
// function-try-block and loops
