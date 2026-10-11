// N3485 focus: 5 [expr], usual arithmetic conversions

static_assert(16777217 == 16777216.0f,
              "the integral operand is converted to float before comparison");

int main()
{
  return 0;
}
