// An inheriting constructor (12.9p8) initializes the class's non-static
// data members as a user-written constructor with an empty
// mem-initializer-list would (12.6.2p8): each member with a brace-or-
// equal-initializer by that initializer, after the base, whether it is
// braced (`T m{40};`, list-initialization by a constructor), copy-
// initialization by a converting constructor (`T m = 41;`) or names
// another member, and whether the member is a class object, a scalar, an
// aggregate or an array; it also does so through a default argument's
// shorter form and in a class template. The object's destruction then
// destroys exactly the members so constructed.
int clock;
int constructed;
int destroyed;

struct Tracked
{
  int id;
  int when;
  Tracked(int i) : id(i), when(++clock) { ++constructed; }
  ~Tracked() { ++destroyed; }
};

struct Base
{
  int when;
  Base(int) : when(++clock) {}
};

struct Braced : Base { using Base::Base; Tracked m{40}; };
struct Converted : Base { using Base::Base; Tracked m = 41; };
struct Scalars : Base { using Base::Base; int k{5}; Tracked m{k}; int j = 6; };
struct Pair { int a; Tracked t; };
struct Aggregates : Base
{
  using Base::Base;
  Pair pair{1, 2};
  Tracked array[2]{{3}, {4}};
  int numbers[3]{7, 8, 9};
};
struct Defaults
{
  int b;
  Defaults(int, int b = 30) : b(b) {}
};
struct Defaulted : Defaults { using Defaults::Defaults; Tracked m{50}; Tracked n = 51; };
template <class B> struct Templated : B { using B::B; Tracked m{60}; int z = 61; };

int main()
{
  {
    Braced braced(1);
    if (constructed != 1 || braced.m.id != 40 || braced.m.when <= braced.when) return 1;
  }
  if (destroyed != 1) return 2;
  {
    Converted converted(1);
    if (constructed != 2 || converted.m.id != 41) return 3;
  }
  if (destroyed != 2) return 4;
  {
    Scalars scalars(1);
    if (constructed != 3 || scalars.k != 5 || scalars.m.id != 5 || scalars.j != 6) return 5;
  }
  if (destroyed != 3) return 6;
  {
    Aggregates aggregates(1);
    if (constructed != 6 || aggregates.pair.a != 1 || aggregates.pair.t.id != 2) return 7;
    if (aggregates.array[0].id != 3 || aggregates.array[1].id != 4) return 8;
    if (aggregates.numbers[0] != 7 || aggregates.numbers[2] != 9) return 9;
  }
  if (destroyed != 6) return 10;
  {
    Defaulted defaulted(1);
    if (constructed != 8 || defaulted.b != 30 || defaulted.m.id != 50 || defaulted.n.id != 51) return 11;
  }
  if (destroyed != 8) return 12;
  {
    Templated<Base> templated(1);
    if (constructed != 9 || templated.m.id != 60 || templated.z != 61) return 13;
  }
  if (destroyed != 9) return 14;
  return 0;
}
