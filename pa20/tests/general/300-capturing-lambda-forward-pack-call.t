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
  return value + static_cast<int>(mapped);
}

template<class... Args>
int run(Args&&... args)
{
  int captured = 1;
  auto lambda = [&captured](Args&&... args2)
  {
    return captured + sink(std::forward<Args>(args2)...);
  };
  return lambda(std::forward<Args>(args)...);
}

int main()
{
  int value = 2;
  unsigned long mapped = 3;
  return run(value, mapped) - 6;
}
