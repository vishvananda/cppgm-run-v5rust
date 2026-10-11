template<bool Value>
struct bool_constant
{
  static constexpr bool value = Value;
};

template<class T>
struct traits
{
  static constexpr bool enabled()
  {
    return sizeof(T) == sizeof(int);
  }
};

template<class T>
struct holder
{
  typedef traits<T> trait_type;
  typedef bool_constant<trait_type::enabled()> type;
};

static_assert(holder<int>::type::value, "int enabled");
static_assert(!holder<char>::type::value, "char disabled");

int main()
{
  return holder<int>::type::value && !holder<char>::type::value ? 0 : 1;
}
