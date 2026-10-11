using size_t = decltype(sizeof(0));

template <class T>
struct __count {};

template <bool B, class T = int>
using __enable_if_t = T;

struct S {
  static const size_t _Dt = 8;

  template <size_t __count, __enable_if_t<__count < _Dt, int> = 0>
  static int f(int x) { return x << __count; }
};

int main() { return S::f<1>(3); }
