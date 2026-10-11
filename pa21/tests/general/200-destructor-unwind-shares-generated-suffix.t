int order;

struct failure {};

struct First
{
  ~First() { order = order * 10 + 1; }
};

struct Second
{
  ~Second() { order = order * 10 + 2; }
};

struct Throwing
{
  ~Throwing() noexcept(false)
  {
    order = order * 10 + 3;
    throw failure();
  }
};

struct Owner
{
  First first;
  Second second;
  Throwing throwing;

  ~Owner() noexcept(false) { order = 4; }
};

int main()
{
  try {
    Owner owner;
  } catch(failure const &) {
    return order == 4321 ? 0 : 1;
  }
  return 2;
}
