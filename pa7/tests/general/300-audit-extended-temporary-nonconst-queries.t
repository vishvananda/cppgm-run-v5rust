const int& value = 3;
int*const& pointer = nullptr;
int f() { return __builtin_constant_p(const_cast<int&>(value)) + __builtin_constant_p(const_cast<int*&>(pointer)); }
