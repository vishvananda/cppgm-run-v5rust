// N3485 15.1p2, 6.6p2, 15.2p1: when a return, goto, break or continue leaves a
// try block, the objects whose scope it leaves are destroyed, and an exception
// a destructor throws then is caught not by the handlers of the try block
// already left but by an enclosing one; each function runs without a throw,
// with the throw on the jump and on the normal path
