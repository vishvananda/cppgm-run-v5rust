// VALIDATION: compile-pass
// N3485 focus: 7.1.5 [dcl.constexpr], 13.5 [over.oper]

template<class T>
struct quantity
{
  T value;

  constexpr explicit quantity(T v) : value(v)
  {
  }
};

template<class T>
constexpr bool operator<(quantity<T> left, quantity<T> right)
{
  return left.value < right.value;
}

constexpr bool compare_quantities()
{
  return quantity<int>(1) < quantity<int>(2);
}

static_assert(compare_quantities(), "constexpr comparison finds operator template");

int main()
{
  return compare_quantities() ? 0 : 1;
}
