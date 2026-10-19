// expect reject
// N3485 13.3.3.1.2p1, 8.5.4p7: a runtime int-to-byte conversion narrows.
struct Value { operator int() const { return 3; } };
int main()
{
  unsigned char narrowed{Value{}};
  return narrowed;
}
