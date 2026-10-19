// expect reject
// N3485 13.3.3.1.2p1, 8.5.4p7: check the second standard conversion.
struct Value { operator double() const { return 1.0; } };
int main()
{
  float narrowed{Value{}};
  return (int)narrowed;
}
