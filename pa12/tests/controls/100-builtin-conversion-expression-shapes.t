// N3485 13.3.1.2p7: builtin operators use a selected conversion on the
// original operand, including member and call expressions and parentheses.
// expect exit 0
int conversions;
struct Number {
  operator int() { ++conversions; return 2; }
};
struct Holder { Number value; };
Number make_number() { return Number(); }
int main()
{
  Number value;
  Holder holder;
  int sum = (value + 1) + (((value)) + 1) + (holder.value + 1)
          + (make_number() + 1) + (+value);
  return sum == 14 && conversions == 5 ? 0 : 1;
}
