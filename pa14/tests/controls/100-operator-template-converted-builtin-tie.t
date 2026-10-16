// N3485 13.3.1.2p3, 13.3.3p1: the builtin candidate survives a template
// specialization with the same parameter list and wins tied conversions.
// expect exit 0
struct Number { operator int() const { return 1; } };
template<class T> int operator+(int, T)
{
  return T::missing;
}
int main()
{
  Number value;
  return value + 1 == 2 ? 0 : 1;
}
