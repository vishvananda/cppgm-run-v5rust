// Function-template redeclaration matching must preserve cv nested under a
// reference. Otherwise the disabled T& overload can absorb the T const&
// overload and leave no viable candidate during instantiation.
template<bool B>
struct disable_if {
  typedef int type;
};

template<>
struct disable_if<true> {};

template<class T>
struct is_const {
  static const bool value = false;
};

template<class T>
struct is_const<T const> {
  static const bool value = true;
};

template<class T>
typename disable_if<is_const<T>::value>::type pick(T&);

template<class T>
int pick(T const&);

template<class T>
struct caller {
  static int call(T& value)
  {
    return pick(value);
  }
};

template<class T>
typename disable_if<is_const<T>::value>::type pick(T&)
{
  return 1;
}

template<class T>
int pick(T const&)
{
  return 2;
}

int main()
{
  int const value = 0;
  return caller<int const>::call(value) == 2 ? 0 : 1;
}
