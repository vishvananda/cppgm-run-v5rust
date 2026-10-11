// VALIDATION: a friend function template declared with an alias-template
// result and defined at namespace scope with the same result under other
// parameter names is one template; the definition keeps the friend's
// access to the private constructor.

template <bool Condition, class T = void>
struct enable_if
{
};

template <class T>
struct enable_if<true, T>
{
  typedef T type;
};

template <class T>
struct is_array
{
  static const bool value = false;
};

template <class T>
struct is_array<T[]>
{
  static const bool value = true;
};

template <class T>
using non_array = typename enable_if<!is_array<T>::value, T>::type;

template <class T>
struct allocation_tag
{
};

template <class T>
struct owner
{
  int argument_count;

  template <class Y, class... Args>
  friend owner<non_array<Y> > make_owner(Args&&... args);

private:
  template <class A, class... Args>
  owner(allocation_tag<A>, Args&&...) : argument_count(sizeof...(Args)) {}
};

template <class T, class... Args>
inline owner<non_array<T> > make_owner(Args&&... args)
{
  return owner<T>(allocation_tag<int>(), static_cast<Args&&>(args)...);
}

int main()
{
  owner<int> two = make_owner<int>(1, 2);
  owner<long> three = make_owner<long>(1, 2, 3);
  if (two.argument_count != 2) return 1;
  if (three.argument_count != 3) return 2;
  return 0;
}
