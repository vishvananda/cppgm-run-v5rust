// Deferred mem-initializers must retain the validation of later subobjects.
struct Wrapper {
  int value;
  int* pointer;
  Wrapper() : value(__builtin_unprovided_member()), pointer(1) {}
};
