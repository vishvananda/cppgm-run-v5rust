// VALIDATION: compile-pass
// N3485 focus: 5 [expr], 7.1.5 [dcl.constexpr]

struct value_holder
{
  long long value;

  constexpr explicit value_holder(long long v) : value(v)
  {
  }

  constexpr bool matches() const
  {
    return value == -100;
  }
};

constexpr bool signed_widen_equal()
{
  return value_holder(-100).matches();
}

static_assert(signed_widen_equal(), "signed int operand sign-extends to long long");

int main()
{
  return signed_widen_equal() ? 0 : 1;
}
