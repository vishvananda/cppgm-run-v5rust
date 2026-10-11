namespace std {
template <class T> struct remove_reference { typedef T type; };
template <class T> struct remove_reference<T&> { typedef T type; };
template <class T> struct remove_reference<T&&> { typedef T type; };

template <class T>
T&& forward(typename remove_reference<T>::type& t)
{
  return static_cast<T&&>(t);
}

template <class T>
T&& forward(typename remove_reference<T>::type&& t)
{
  return static_cast<T&&>(t);
}
}

int sink(const char (&text)[2], int value)
{
  return text[0] + value;
}

template <class... Args>
int run(Args&&... args)
{
  auto lambda = [](Args&&... args2)
  {
    return sink(std::forward<Args>(args2)...);
  };
  return lambda(std::forward<Args>(args)...);
}

int main()
{
  return run("x", 1) - ('x' + 1);
}
