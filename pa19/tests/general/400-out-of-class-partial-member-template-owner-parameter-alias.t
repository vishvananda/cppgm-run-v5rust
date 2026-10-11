// VALIDATION: compile-pass
// An out-of-class member-template definition may rename its enclosing partial
// specialization parameter. Paired cv overloads must retain both that owner
// alias and their distinct function signatures when attached to an instance.
template<class T>
struct decay
{
  typedef T type;
};

template<class T>
struct decay<T const &>
{
  typedef T type;
};

template<class E, class U>
E const * peek(U const &)
{
  static E value = 7;
  return &value;
}

template<class E, class U>
E * peek(U &)
{
  static E value = 7;
  return &value;
}

template<class E, bool>
struct traits;

template<class E>
struct traits<E, false>
{
  typedef typename decay<E>::type error_type;

  template<class U>
  static error_type const * check(U const &);

  template<class U>
  static error_type * check(U &);
};

template<class A>
template<class U>
typename traits<A, false>::error_type const *
traits<A, false>::check(U const & value)
{
  return peek<typename decay<A>::type>(value);
}

template<class A>
template<class U>
typename traits<A, false>::error_type * traits<A, false>::check(U & value)
{
  return peek<typename decay<A>::type>(value);
}

int main()
{
  int value = 0;
  int const & const_value = value;
  int * direct = traits<int const &, false>::check(value);
  int const * const_direct = traits<int const &, false>::check(const_value);
  return *direct == 7 && *const_direct == 7 ? 0 : 1;
}
