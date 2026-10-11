// VALIDATION: compile-pass
// A small derived class with a user-provided copy constructor is non-trivial
// for the purposes of calls, so it is passed by address: the parameter is the
// caller's copy, and copying it passes that address as the constructor source.

struct Base
{
  Base(void * first, void * second) : first(first), second(second) {}
  Base(const Base &) = default;

  void * first;
  void * second;
};

struct Pair : Base
{
  Pair(void * first, void * second) : Base(first, second) {}
  Pair(const Pair & other) : Base(other) {}
};

int seen = 0;

void use(const Pair & value)
{
  if(value.first == 0 && value.second == reinterpret_cast<void *>(16)) {
    seen = 1;
  }
}

void check(Pair value)
{
  Pair copy(value);
  use(copy);
}

int main()
{
  Pair input(0, reinterpret_cast<void *>(16));
  check(input);
  return seen == 1 ? 0 : 1;
}
