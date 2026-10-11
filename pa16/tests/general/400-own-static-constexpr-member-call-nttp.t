template<bool B>
struct bool_constant {
  static constexpr bool value = B;
};

template<class T>
struct holder {
  static constexpr bool enabled() { return sizeof(T) == sizeof(int); }
  typedef bool_constant<enabled()> type;
};

static_assert(holder<int>::type::value, "int enabled");
static_assert(!holder<char>::type::value, "char disabled");

int main() {
  return holder<int>::type::value ? 0 : 1;
}
