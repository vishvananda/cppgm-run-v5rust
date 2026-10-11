namespace std {
template<class T, T v>
struct integral_constant {
  static const T value = v;
  typedef integral_constant type;
};

typedef integral_constant<bool, true> true_type;
typedef integral_constant<bool, false> false_type;

template<bool B>
struct selected {
  static const int value = 0;
};

template<>
struct selected<true> {
  static const int value = 7;
};

template<class T>
struct __is_tuple_like : false_type {};

template<class T>
struct __not_ : integral_constant<bool, !bool(T::value)> {};

union _Any_data {};
}

typedef std::selected<
    bool(std::__not_<std::__is_tuple_like<std::_Any_data> >::value)> result;

int main()
{
  return result::value == 7 ? 0 : 1;
}
