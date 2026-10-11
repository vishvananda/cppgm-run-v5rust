namespace ns {
struct marker {};

template<class T0 = marker, class T1 = marker>
struct tuple {};
}

struct tag {};

template<class T>
struct box {
  int code;

  template<template<class, class> class Tuple>
  box(Tuple<T, ::ns::marker>)
  {
    code = 1;
  }

  template<template<class, class> class Tuple>
  box(tag, Tuple<T, ::ns::marker>, Tuple< ::ns::marker, ::ns::marker >)
  {
    code = 2;
  }

  int value()
  {
    return code;
  }
};

int main()
{
  box<int> first = box<int>(ns::tuple<int>());
  box<int> second = box<int>(tag(), ns::tuple<int>(), ns::tuple<>());
  return first.value() + second.value() - 3;
}
