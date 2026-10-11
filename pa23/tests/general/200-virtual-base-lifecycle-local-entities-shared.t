// A constructor's and a destructor's local classes and lambdas are one
// class and one closure type for the complete-object entry and the
// base-subobject entry a class with a virtual base gives each (9.8,
// 5.1.2p3); Itanium names them in C1 and D1.
int calls = 0;
int destroyed = 0;
int (*seen[2])() = { 0, 0 };

struct shared
{
  int tag;
  shared() : tag(1) {}
};

struct counter : virtual shared
{
  int value;

  explicit counter(int slot)
  {
    struct step { static int next(int v) { calls = calls + 1; return v + calls; } };
    auto source = []() -> int { return 4; };
    int (*pointer)() = source;
    seen[slot] = pointer;
    value = step::next(source());
  }

  ~counter()
  {
    struct tally { static void hit() { destroyed = destroyed + 1; } };
    auto again = [] { destroyed = destroyed + 10; };
    tally::hit();
    again();
  }
};

struct derived : counter
{
  derived() : counter(1) {}
};

int main()
{
  {
    counter whole(0);
    derived part;
    if (whole.value != 5 || part.value != 6) return 10;
    if (seen[0] != seen[1]) return 11;
  }
  return destroyed == 22 ? 0 : 12;
}
