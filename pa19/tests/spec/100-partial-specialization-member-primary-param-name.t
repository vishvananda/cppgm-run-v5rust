// N3485 focus: 14.5.5 [temp.class.spec]
// A selected partial specialization uses its own template-parameter scope when
// checking member declarations; primary parameter names are not reserved there.

template <unsigned digits, bool small = (digits <= 9)>
struct known_digits;

template <unsigned digits_>
struct known_digits<digits_, true>
{
  static const unsigned digits = digits_;
  unsigned value;
};

template <unsigned digits_>
struct known_digits<digits_, false>
{
  static const unsigned digits = digits_;
  unsigned long long value;
};

template <class U>
bool accepts_known_digits(int current, U next)
{
  return current == 1 && next.value == 10 && U::digits == 10;
}

int main()
{
  return accepts_known_digits(1, known_digits<10>{10}) ? 0 : 1;
}
