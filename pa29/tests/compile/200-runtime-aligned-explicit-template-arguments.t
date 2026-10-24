void *invalid(int *p) { return __builtin_assume_aligned<int>(p, 8); }
