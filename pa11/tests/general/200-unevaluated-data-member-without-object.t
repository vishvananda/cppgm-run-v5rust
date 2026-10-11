// A non-static data member may be named without an object in an
// unevaluated operand (N3485 5.1.1p13): decltype and sizeof of
// `Holder::pair` need no `this`.
struct Holder
{
  int pair;
  char tag;
};

decltype(Holder::pair) value = 4;

int width()
{
  return int(sizeof(Holder::pair) + sizeof(Holder::tag));
}

int main()
{
  return value == 4 && width() == 5 ? 0 : 1;
}
