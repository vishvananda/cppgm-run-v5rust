// VALIDATION: "name<>()" inside a member function body names the enclosing
// class's member template even when another class declared a non-template
// member of the same name earlier in the translation unit, and even when
// the member template is declared later in the class.

struct other_detector
{
  static constexpr bool enable_implicit_default() { return false; }
};

template <class T>
struct is_default_constructible
{
  static const bool value = true;
};

template <class T1, class T2>
struct pair_like
{
  struct check_args
  {
    template <class...>
    static constexpr bool enable_explicit_default()
    {
      return is_default_constructible<T1>::value &&
             is_default_constructible<T2>::value &&
             !enable_implicit_default<>();
    }

    template <class...>
    static constexpr bool enable_implicit_default() { return true; }
  };

  T1 first;
  T2 second;
};

int main()
{
  if (pair_like<int, long>::check_args::enable_explicit_default<>()) return 1;
  if (!pair_like<int, long>::check_args::enable_implicit_default<>()) return 2;
  if (other_detector::enable_implicit_default()) return 3;
  return 0;
}
