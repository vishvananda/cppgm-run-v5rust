// expect exit 0
// N3485 8.5.4p7: reuse the constant interpreter for variables and calls.
constexpr double fraction() { return 1.5; }
constexpr long double source = 2.25L;
struct Value {
  constexpr operator double() const { return 3.5; }
};
struct IntegralValue {
  constexpr operator long() const { return 9; }
};
int main()
{
  float first{fraction()};
  double second{source};
  float third{Value{}};
  unsigned char fourth{IntegralValue{}};
  return first == 1.5f && second == 2.25 && third == 3.5f && fourth == 9 ? 0 : 1;
}
