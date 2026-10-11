// An anonymous union in an unnamed namespace need not be declared static;
// only one in a named or the global namespace must be (N3485 9.5p5).
namespace
{
  union
  {
    int whole;
    char bytes[4];
  };
}

int main()
{
  whole = 0;
  bytes[0] = 7;
  return whole != 0 ? 0 : 1;
}
