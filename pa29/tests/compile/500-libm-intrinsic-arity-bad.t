// A libm intrinsic rejects a call with the wrong number of operands.
double wrong(double x) { return __builtin_sqrt(x, x); }

int main() { return 0; }
