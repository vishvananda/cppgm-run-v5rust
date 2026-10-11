int live;

struct Value
{
  Value() { ++live; }
  ~Value() { --live; }
};

struct Base
{
  Base(const Value &) {}
};

struct Derived : Base
{
  Derived() : Base(Value()) {}
};

int main()
{
  Derived object;
  return live;
}
