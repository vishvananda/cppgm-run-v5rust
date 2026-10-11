struct Tag { int value; };
struct Base { virtual int f() const = 0; };
struct Mid : Base { virtual int g() const { return 1; } };

struct Derived : Tag, Mid {
  int f() const { return 2; }
  int g() const { return 7; }
};

int main()
{
  Derived value;
  Mid * view = &value;
  return view->g() == 7 ? 0 : 1;
}
