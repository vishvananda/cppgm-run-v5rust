// A local class's default member initializer has its own analysis context.
inline int wrapper() {
  __builtin_unprovided_outer();
  struct Local { int value = __builtin_unprovided_local(); };
  return 0;
}
