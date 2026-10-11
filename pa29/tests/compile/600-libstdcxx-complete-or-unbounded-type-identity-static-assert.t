namespace std {
typedef decltype(sizeof(0)) size_t;

template <class T, T V>
struct integral_constant {
  static constexpr T value = V;
  constexpr operator T() const noexcept { return value; }
};

template <class T>
struct __type_identity {
  typedef T type;
};

template <class T, size_t = sizeof(T)>
constexpr integral_constant<bool, true>
__is_complete_or_unbounded(__type_identity<T>)
{
  return {};
}

template <class T>
struct is_trivially_copyable
  : integral_constant<bool, __is_trivially_copyable(T)> {
  static_assert(std::__is_complete_or_unbounded(__type_identity<T>{}),
                "template argument must be complete");
};
}

struct Complete {
  int value;
};

bool probe = std::is_trivially_copyable<Complete>::value;

int main()
{
  return probe ? 1 : 0;
}
