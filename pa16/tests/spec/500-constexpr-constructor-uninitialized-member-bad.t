// N3485 focus: 7.1.5 [dcl.constexpr]

struct invalid_object
{
  int value;

  constexpr invalid_object()
  {
  }
};

int main()
{
  return 0;
}
