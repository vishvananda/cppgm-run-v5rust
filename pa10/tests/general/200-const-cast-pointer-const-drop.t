int **drop_const_pointer(int * const *p) {
  return const_cast<int **>(p);
}
