template<class Sig>
struct unary {
  static const int value = 0;
};

template<class R, class A1>
struct unary<R(A1)> {
  static const int value = 1;
};

template<class Sig>
struct fn;

template<class R, class... Args>
struct fn<R(Args...)> : unary<R(Args...)> {};

int main() {
  return (fn<void(int)>::value != 1) ||
         (fn<void(int, float)>::value != 0);
}
