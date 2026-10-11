struct Base
{
  virtual int value() const { return 7; }
};

struct Wrapper
{
  Base * ptr;

  Wrapper(Base & value) : ptr(&value) {}

  operator const Base &() const { return *ptr; }
};

int main()
{
  Base base;
  Wrapper wrapper(base);
  const Base & ref = wrapper;
  if(ref.value() != 7)
    return 1;

  Wrapper wrappers[1] = {Wrapper(base)};
  for(const Base & item : wrappers) {
    if(item.value() != 7)
      return 2;
  }
  return 0;
}
