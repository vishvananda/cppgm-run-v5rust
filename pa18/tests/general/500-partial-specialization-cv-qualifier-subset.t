// VALIDATION: compile-pass

template<typename A, typename B>
struct is_same
{
  static const bool value = false;
};

template<typename A>
struct is_same<A, A>
{
  static const bool value = true;
};

template<class T>
struct remove_const
{
  typedef T type;
};

template<class T>
struct remove_const<const T>
{
  typedef T type;
};

template<class T>
struct remove_volatile
{
  typedef T type;
};

template<class T>
struct remove_volatile<volatile T>
{
  typedef T type;
};

template<class T>
struct holder
{
  typedef T element_type;
  typedef typename remove_volatile<
      typename remove_const<T>::type>::type value_type;
};

static_assert(is_same<holder<const volatile int>::element_type,
                      const volatile int>::value,
              "element type preserves cv");
static_assert(is_same<holder<const volatile int>::value_type,
                      int>::value,
              "value type strips cv through partial specializations");

int main()
{
  return 0;
}
