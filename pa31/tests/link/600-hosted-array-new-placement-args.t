// N3485 5.3.4p11/p13: an array new-expression passes its new-placement
// arguments: new (buf) int[2] uses buf, new (std::nothrow) int[4] calls the
// nothrow operator new[]
