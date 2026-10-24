// Name-only checking of one full-expression must not suppress the next's typing.
inline int wrapper() {
  __builtin_unprovided_neighbor();
  int* pointer = 1;
  return 0;
}
