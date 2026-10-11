int order;

struct failure {};

struct Base
{
  ~Base() { order = order * 10 + 2; }
};

struct Member
{
  ~Member() noexcept(false)
  {
    order = order * 10 + 1;
    throw failure();
  }
};

struct Derived : Base
{
  Member member;
  ~Derived() noexcept(false) { order = order * 10 + 3; }
};

int main()
{
  try {
    Derived value;
  } catch(failure const &) {
    return order == 312 ? 0 : 1;
  }
  return 2;
}
