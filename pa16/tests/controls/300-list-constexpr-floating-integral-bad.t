// expect reject
// N3485 8.5.4p7: floating-to-integer conversion always narrows.
struct Value { constexpr operator double() const { return 3.0; } };
int main()
{
  int narrowed{Value{}};
  return narrowed;
}
