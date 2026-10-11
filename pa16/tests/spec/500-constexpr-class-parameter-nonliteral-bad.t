// N3485 focus: 3.9 [basic.types], 7.1.5 [dcl.constexpr]

struct nonliteral
{
  nonliteral();
};

constexpr int invalid_parameter(nonliteral)
{
  return 0;
}

int main()
{
  return 0;
}
