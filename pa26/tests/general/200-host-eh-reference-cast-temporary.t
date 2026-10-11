// N3485 5.2.9p4, 5.4p4, 12.2p5: a cast to a reference type whose operand's
// binding makes a temporary (a converting constructor's, a conversion
// function's, a converted scalar) designates it; a reference bound to the
// cast's result extends it as one bound to the operand would (as g++ and clang
// do): a local reference until its scope ends (also when an exception leaves
// it), a static or namespace-scope one until the program ends
