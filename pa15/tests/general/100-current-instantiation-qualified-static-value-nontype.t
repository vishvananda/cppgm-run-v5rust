// Regression: a current-instantiation static member value can feed a non-type
// template argument while the current class template is still collecting members.

template<bool B>
struct bool_constant
{
  static const bool value = B;
};

template<class T>
struct trait
{
  static const bool value = false;
  typedef bool_constant<trait<T>::value> type;
};

struct sequence {};

static_assert(!trait<sequence>::type::value, "qualified current value");

int main()
{
  return 0;
}
