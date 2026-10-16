// Enum constant evaluation consumes semantic overload and sizeof facts.
int select(int);
long select(char);

enum sizes {
  integer_size = sizeof(select(0)),
  long_size = sizeof(select('x')),
  next_size = integer_size + 1
};

int result() {
  return integer_size == sizeof(int) && long_size == sizeof(long)
      && next_size == sizeof(int) + 1;
}
