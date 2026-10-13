int x, y;
int a[2];
int* p;
int& r = x;
int f() { return __builtin_constant_p(*&x = 3) + __builtin_constant_p(a[1] = 3) + __builtin_constant_p((true ? x : y) = 3) + __builtin_constant_p((x,y) = 3) + __builtin_constant_p(r = 3) + __builtin_constant_p(p = nullptr) + __builtin_constant_p(*&p = nullptr) + __builtin_constant_p(x = {}) + __builtin_constant_p(x = {3}); }
