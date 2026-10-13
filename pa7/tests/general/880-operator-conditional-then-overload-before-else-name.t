// Scalar analysis failure ordering, compared with the frozen C++ owner.
void f(int);void f(long);void g(){true?f:missing;}
