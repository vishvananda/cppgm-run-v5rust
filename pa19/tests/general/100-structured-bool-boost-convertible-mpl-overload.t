// Reduced from Boost.Xpressive has_fold_case<Traits>() overload selection.
// A Boost-style is_convertible false result converts to mpl::bool_<false>,
// not to the true wrapper.

template<bool B>
struct bool_
{
  static const bool value = B;
  typedef bool_ type;
  constexpr operator bool() const { return B; }
};

template<bool B>
bool const bool_<B>::value;

template<class T, T V>
struct integral_constant
{
  static const T value = V;
  typedef integral_constant type;

  operator const bool_<V>&() const
  {
    static const char data[sizeof(long)] = {0};
    const void *pdata = data;
    return *static_cast<const bool_<V> *>(pdata);
  }

  constexpr operator T() const { return V; }
};

template<class T, T V>
T const integral_constant<T, V>::value;

typedef char yes_type;
typedef int no_type;

struct any_conversion
{
  template<class T> any_conversion(const volatile T&);
  template<class T> any_conversion(const T&);
  template<class T> any_conversion(volatile T&);
  template<class T> any_conversion(T&);
};

template<class T>
struct checker
{
  static no_type _m_check(any_conversion ...);
  static yes_type _m_check(T, int);
};

template<class From, class To>
struct is_convertible_impl
{
  typedef From &&rvalue_type;
  static From &from;
  static const bool value =
      sizeof(checker<To>::_m_check(static_cast<rvalue_type>(from), 0)) ==
      sizeof(yes_type);
};

template<class From, class To>
struct is_convertible
  : integral_constant<bool, is_convertible_impl<From, To>::value>
{
};

struct version_1_tag {};
struct version_2_tag : version_1_tag {};
struct case_fold_tag : version_1_tag {};

template<class Traits>
struct has_fold_case
  : is_convertible<typename Traits::version_tag *, case_fold_tag *>
{
};

struct traits
{
  typedef version_2_tag version_tag;
};

char pick(bool_<false>);
int pick(bool_<true>);

static_assert(sizeof(pick(has_fold_case<traits>())) == sizeof(char),
              "expected false wrapper");

int main()
{
  return 0;
}
