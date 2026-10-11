typedef unsigned long T;

bool __is_type_name_unique(T v) { return !(v & 1); }

bool f(T v) { return __is_type_name_unique(v); }
