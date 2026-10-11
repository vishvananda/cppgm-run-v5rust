namespace std {
template<class T> struct remove_reference { typedef T type; };
template<class T> struct remove_reference<T&> { typedef T type; };
template<class T> struct remove_reference<T&&> { typedef T type; };

template<class T>
T&& forward(typename remove_reference<T>::type& t)
{
  return static_cast<T&&>(t);
}

template<class T>
T&& forward(typename remove_reference<T>::type&& t)
{
  return static_cast<T&&>(t);
}
}

int sink(int& value, unsigned long& mapped)
{
  mapped += 4;
  return value + static_cast<int>(mapped);
}

template<class... Args>
int run(Args&&... args)
{
  auto lambda = [&]
  {
    return sink(std::forward<Args>(args)...);
  };
  return lambda();
}

int main()
{
  int value = 2;
  unsigned long mapped = 3;
  int result = run(value, mapped);
  return result == 9 && mapped == 7 ? 0 : 1;
}
