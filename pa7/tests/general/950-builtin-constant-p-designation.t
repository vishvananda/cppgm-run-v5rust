// A query builtin has no function value outside its intrinsic call form.
int (*pointer)(...) = __builtin_constant_p;
