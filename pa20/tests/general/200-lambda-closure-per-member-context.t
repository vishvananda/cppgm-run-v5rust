// A lambda expression is one closure type in its context (N3485 5.1.2/3):
// a default member initializer's whichever constructor initializes the
// member, a class template static data member initializer's once per
// specialization, and an in-class default argument's wherever a call uses
// it.
struct holder
{
  int (*make)() = [] { return 1; };
  int base;
  holder() : base(0) {}
  explicit holder(int b) : base(b) {}
};

template<class T> struct width { static int (*get)(); };
template<class T> int (*width<T>::get)() = [] { return int(sizeof(T)); };

int (*first_default)() = 0;

struct chooser
{
  static int call(int (*f)() = [] { return 2; })
  {
    if (!first_default) first_default = f;
    return f == first_default ? f() : 100;
  }
};

int early = chooser::call();

int main()
{
  holder a;
  holder b(5);
  if (a.make != b.make) return 10;
  if (width<int>::get() != 4 || width<char>::get() != 1) return 11;
  if (chooser::call() != 2) return 12;
  return a.make() + early + b.base == 8 ? 0 : 13;
}
