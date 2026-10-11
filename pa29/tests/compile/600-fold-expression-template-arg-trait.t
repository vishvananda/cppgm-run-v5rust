template<class T>
struct wrapper {
};

template<class T>
struct is_wrapped {
  static constexpr bool value = false;
};

template<class T>
struct is_wrapped<wrapper<T> > {
  static constexpr bool value = true;
};

template<class... Ts>
struct all_wrapped {
  static_assert((is_wrapped<wrapper<Ts> >::value && ...), "all wrapped");
};

all_wrapped<int, long> ok;

int main()
{
  return 0;
}
