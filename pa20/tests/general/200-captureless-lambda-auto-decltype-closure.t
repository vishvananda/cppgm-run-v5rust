template<class A, class B>
struct same
{
  static constexpr bool value = false;
};

template<class A>
struct same<A, A>
{
  static constexpr bool value = true;
};

int main()
{
  auto fn = [](int lhs, char rhs) -> int
  {
    return lhs + rhs;
  };
  using F = decltype(fn);

  static_assert(!same<F, int (*)(int, char)>::value,
                "auto preserves a captureless lambda's closure type");

  int (*pointer)(int, char) = fn;
  int (*plus_pointer)(int, char) = +fn;
  return fn(3, 4) == 7 &&
         pointer(4, 3) == 7 &&
         plus_pointer(5, 2) == 7 ? 0 : 1;
}
