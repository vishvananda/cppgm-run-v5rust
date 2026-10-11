template<class F>
struct adaptor {
  static const int value = 0;
};

template<class T, class Object>
struct adaptor<T Object::*> {
  static const int value = 1;
};

template<class Result, class Object, class Arg1>
struct adaptor<Result (Object::*)(Arg1) const> {
  static const int value = 2;
};

template<class Result, class Object, class Arg1>
struct adaptor<Result (Object::*)(Arg1)> {
  static const int value = 3;
};

struct A {
  int get(int const &) const;
};

static_assert(adaptor<int (A::*)(int const &) const>::value == 2, "");

int main()
{
  return adaptor<int (A::*)(int const &) const>::value == 2 ? 0 : 1;
}
