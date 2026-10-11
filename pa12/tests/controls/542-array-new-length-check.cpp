// N3485 5.3.4p7: an array new count that is negative, or whose request
// would exceed the implementation limit, obtains no storage and throws
// std::bad_array_new_length. The count is checked after its conversion to
// std::size_t and before the allocation function is called, through the
// `bad_array_new_length` runtime role; a count type that cannot hold such a
// value needs no check. Valid counts allocate normally.
struct Destructible {
  long value;
  Destructible() : value(5) {}
  ~Destructible() {}
};

int* make_signed(int count) { return new int[count]; }
char* make_sized(unsigned long count) { return new char[count]; }
Destructible* make_class(long count) { return new Destructible[count]; }
int* make_unsigned(unsigned count) { return new int[count]; }

int main() {
  int* a = make_signed(3);
  char* b = make_sized(4);
  Destructible* c = make_class(2);
  int* d = make_unsigned(5);
  a[2] = 1;
  b[3] = 2;
  d[4] = 3;
  const int status = a[2] + b[3] + d[4] + static_cast<int>(c[1].value) == 11 ? 0 : 1;
  delete[] a;
  delete[] b;
  delete[] c;
  delete[] d;
  return status;
}
