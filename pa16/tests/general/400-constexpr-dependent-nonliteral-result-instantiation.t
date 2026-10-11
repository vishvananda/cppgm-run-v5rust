template<class T>
struct guard {
  guard() {}
  ~guard() {}
};

template<class T>
constexpr guard<T> make_guard(T)
{
  return guard<T>();
}

template<class T>
struct guard_factory {
  static constexpr guard<T> make(T)
  {
    return guard<T>();
  }
};

int main()
{
  make_guard(1);
  guard_factory<int>::make(1);
}
