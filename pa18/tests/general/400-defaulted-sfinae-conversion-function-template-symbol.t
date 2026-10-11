// VALIDATION: compile-pass
// Defaulted SFINAE type arguments remain part of a conversion-function
// template specialization's ABI identity after target-type deduction.

template<bool B, class T = void>
struct enable_if
{
};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

struct yes_type
{
};

struct no_type
{
};

template<class A, class B>
struct is_same
{
  static const bool value = false;
};

template<class A>
struct is_same<A, A>
{
  static const bool value = true;
};

template<class T, unsigned long N>
struct array
{
  int value;
};

template<class T, unsigned long N>
yes_type assign_is_array(const array<T, N> *);

no_type assign_is_array(...);

template<class T>
struct source
{
  template<class Container,
           class = typename Container::iterator>
  operator Container() const
  {
    return Container{1};
  }

  template<class Container,
           class = typename enable_if<
               is_same<yes_type,
                       decltype(assign_is_array((Container *)0))>::value>::type,
           class = void>
  operator Container() const
  {
    return Container{2};
  }
};

int main()
{
  array<int, 2> value = {0};
  value = source<int>();
  return value.value == 2 ? 0 : 1;
}
