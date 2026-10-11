int conditions;
int copies;
int alive;

bool select_left()
{
  ++conditions;
  return false;
}

struct Base { int value; };
struct Derived : Base
{
  Derived() { value = 7; }
  Derived(const Base & input) { value = input.value + 10; }
};

struct Observed
{
  int value;
  Observed(int input) : value(input) { ++alive; }
  Observed(const Observed & other) : value(other.value)
  {
    ++alive;
    ++copies;
  }
  ~Observed() { --alive; }
};

int main()
{
  Base base = {3};
  Derived derived;
  Base copy = select_left() ? base : derived;
  if (copy.value != 7 || conditions != 1) return 1;
  copy.value = 9;
  if (derived.value != 7) return 2;
  {
    Observed first(5);
    Observed second(8);
    Observed copied = select_left() ? first : second;
    if (copied.value != 8 || copies != 1 || alive != 3 || conditions != 2)
      return 3;
  }
  return alive != 0;
}
