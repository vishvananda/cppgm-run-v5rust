unsigned long invalid(int *p) { return __builtin_object_size<int>(p, 0); }
