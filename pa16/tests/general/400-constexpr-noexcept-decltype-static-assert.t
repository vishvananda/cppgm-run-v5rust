// VALIDATION: compile-pass
// Constant evaluation for a noexcept expression reached through decltype must
// materialize constexpr function-template bodies even when ordinary function
// body instantiation is suppressed for the unevaluated operand.

typedef decltype(sizeof(0)) size_t;

template<class T, T v>
struct integral_constant
{
  static const T value = v;
  constexpr operator T() const { return v; }
};

typedef integral_constant<bool, true> true_type;

template<class T>
struct type_identity
{
  typedef T type;
};

template<class T, size_t = sizeof(T)>
constexpr true_type complete_or_unbounded(type_identity<T>)
{
  return true_type();
}

template<class TypeIdentity, class NestedType = typename TypeIdentity::type>
constexpr true_type complete_or_unbounded(TypeIdentity)
{
  return true_type();
}

template<class T>
struct is_nothrow_copy_constructible : true_type
{
  static_assert(complete_or_unbounded(type_identity<T>()), "complete");
};

namespace std
{
  template<class I>
  I niter_base(I it) noexcept(is_nothrow_copy_constructible<I>::value);
}

template<class T>
struct box
{
  static void f(T p)
  {
    using IterBase = decltype(std::niter_base(p));
    IterBase q = p;
    (void)q;
  }
};

int main()
{
  char const * p = 0;
  box<char const *>::f(p);
  return 0;
}
