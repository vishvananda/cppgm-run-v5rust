// VALIDATION: compile-pass
// N3485 focus: 14.6.4 [temp.point], 14.8 [temp.fct.spec]
// A dependent function-template result may name a class specialization whose
// omitted non-type argument is declared in a nested namespace. Substitution
// must reevaluate that default in the class template's declaring scope.

namespace units {

template<class Rep>
struct box
{
  explicit box(Rep input) : value(input) {}
  Rep value;
};

namespace detail {

template<class T>
struct is_box
{
  static constexpr bool value = false;
};

template<class Rep>
struct is_box<box<Rep>>
{
  static constexpr bool value = true;
};

template<class Box, class Scalar,
         bool IsBox = is_box<Scalar>::value>
struct divide_result
{
};

template<class Rep, class Scalar>
struct divide_result<box<Rep>, Scalar, false>
{
  typedef box<Scalar> type;
};

}

template<class Rep, class Scalar>
typename detail::divide_result<box<Rep>, Scalar>::type
operator/(const box<Rep>& input, Scalar scalar)
{
  typedef typename detail::divide_result<box<Rep>, Scalar>::type result_type;
  return result_type(static_cast<Scalar>(input.value) / scalar);
}

}

int main()
{
  units::box<long> input(6);
  units::box<double> result = input / 2.0;
  return result.value == 3.0 ? 0 : 1;
}
