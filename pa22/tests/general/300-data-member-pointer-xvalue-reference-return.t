struct X
{
  int m;
};

int && select_member(X && object, int X::* member)
{
  return static_cast<X &&>(object).*member;
}

int main()
{
  X value = { 42 };
  int X::* member = &X::m;
  int && selected = select_member(static_cast<X &&>(value), member);
  return &selected == &value.m && selected == 42 ? 0 : 1;
}
