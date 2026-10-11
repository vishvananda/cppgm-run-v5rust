// VALIDATION: compile-pass
// N3485 focus: 14.1 [temp.param], 14.3.2 [temp.arg.nontype]

struct pair
{
  int x;
  int y;
};

template<int pair::* PtrToPairMember>
struct foo
{
  int bar(pair& p)
  {
    return p.*PtrToPairMember;
  }
};

int use(pair& p)
{
  foo<&pair::x> fx;
  foo<&pair::y> fy;
  return fx.bar(p) + fy.bar(p);
}

int main()
{
  return 0;
}
