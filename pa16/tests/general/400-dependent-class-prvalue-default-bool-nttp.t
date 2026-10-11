template<typename T, T v>
struct integral_constant {
  static constexpr T value = v;
  constexpr operator T() const noexcept { return value; }
};

template<class T>
struct trait : integral_constant<bool, false> {};

template<>
struct trait<int> : integral_constant<bool, true> {};

template<class T, bool B = trait<T>{}>
struct sink {
  static constexpr bool value = B;
};

static_assert(sink<int>::value,
              "dependent default bool NTTP should fold class prvalue to true");
static_assert(!sink<char>::value,
              "dependent default bool NTTP should fold class prvalue to false");
