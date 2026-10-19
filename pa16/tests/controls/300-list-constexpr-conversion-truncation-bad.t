// expect reject
// N3485 8.5.4p7: inspect 300 before conversion to unsigned char truncates it.
struct Value { constexpr operator int() const { return 300; } };
int main()
{
  unsigned char narrowed{Value{}};
  return narrowed;
}
