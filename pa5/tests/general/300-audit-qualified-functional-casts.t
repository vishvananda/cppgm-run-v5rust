struct C { typedef int word; };
int value;
void f()
{
  ::C();
  ::C{};
  typename C::word();
  typename C::word{};
  decltype(value)(1);
  decltype(value){2};
}
