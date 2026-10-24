bool invalid(int *p) { return __builtin_add_overflow<int>(1, 2, p); }
