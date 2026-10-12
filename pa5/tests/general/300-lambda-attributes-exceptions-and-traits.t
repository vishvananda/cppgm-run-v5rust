struct C {};
template<class... Args>
void parse_lambdas(Args... args) {
  int x;
  auto first = [&, args..., this](int C) __attribute__((noinline)) mutable noexcept(C) [[gnu::used]] -> int {
    C * value;
    return C;
  };
  auto second = [=, &x]() __attribute__((pure)) throw(int, C*) [[gnu::noinline]] -> C* { return nullptr; };
  auto third = []() noexcept([] { return 1; }()) -> decltype([] { return 1; }()) { return 1; };
  auto fourth = [](int x = [] { return 1; }()) { return sizeof([] { return 1; }); };
}
