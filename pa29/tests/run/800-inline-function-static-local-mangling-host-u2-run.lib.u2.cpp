// partner TU (the mixed check builds it with g++)
__attribute__((always_inline)) inline int counter() { static int n = 0; return ++n; }
int other() { return counter(); }
