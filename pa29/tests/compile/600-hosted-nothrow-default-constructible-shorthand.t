namespace std {
template<class T> struct is_nothrow_default_constructible {
  static const bool value = __is_nothrow_constructible(T);
};
template<class T> struct is_nothrow_copy_constructible {
  static const bool value = __is_nothrow_constructible(T, const T&);
};
template<class T> struct is_nothrow_move_constructible {
  static const bool value = __is_nothrow_constructible(T, T&&);
};
}

struct selector
{};

struct throwing_default
{
  throwing_default() noexcept(false) {}
};

struct throwing_copy
{
  throwing_copy() {}
  throwing_copy(const throwing_copy&) noexcept(false) {}
};

static_assert(std::is_nothrow_default_constructible<selector>::value,
              "trivial selector is nothrow default constructible");
static_assert(!std::is_nothrow_default_constructible<throwing_default>::value,
              "throwing default constructor is not nothrow");
static_assert(std::is_nothrow_copy_constructible<selector>::value,
              "trivial selector is nothrow copy constructible");
static_assert(std::is_nothrow_move_constructible<selector>::value,
              "trivial selector is nothrow move constructible");
static_assert(!std::is_nothrow_copy_constructible<throwing_copy>::value,
              "throwing copy constructor is not nothrow");

int main()
{
  return 0;
}
