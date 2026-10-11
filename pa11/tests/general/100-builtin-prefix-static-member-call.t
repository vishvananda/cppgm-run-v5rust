typedef unsigned long T;

struct S {
  static bool __is_type_name_unique(T v) { return !(v & 1); }
  static bool f(T v) { return __is_type_name_unique(v); }
};
