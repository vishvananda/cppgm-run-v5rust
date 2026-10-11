// An explicit member-function-template specialization can deduce its template
// argument from one parameter while another parameter is a non-deduced alias.

template<bool Condition, class T = void>
struct conditional_type {};

template<class T>
struct conditional_type<true, T> { typedef T type; };

template<bool Condition, class T = int>
using constraint_t = typename conditional_type<Condition, T>::type;

struct property {};
struct base {};

namespace traits {
template<class T, class Property>
struct require_member { static const bool is_valid = true; };
}

struct executor {
  typedef base base_type;

  template<class Property>
  executor require(Property const&,
                   constraint_t<
                     traits::require_member<const base_type&,
                                            const Property&>::is_valid> = 0) const
  {
    return executor();
  }
};

template<>
executor executor::require(property const&, int) const;

template<>
executor executor::require(property const&, int) const
{
  return executor();
}

int main()
{
  executor value;
  property p;
  value.require(p);
}
