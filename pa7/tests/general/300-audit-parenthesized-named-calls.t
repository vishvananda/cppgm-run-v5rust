int function(int = 1);
int (*pointer)(int);
int (&reference)(int) = function;
int f() { return ((function))() + ((pointer))(1) + ((reference))(2) + (__builtin_constant_p)(3); }
