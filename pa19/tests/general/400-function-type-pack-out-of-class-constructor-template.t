template<class T>
struct Function;

template<class R, class... Args>
struct Function<R(Args...)> {
  template<class F, class = void>
  Function(F f);
};

template<class R, class... Args>
template<class F, class>
Function<R(Args...)>::Function(F f) {}

int main() {
  Function<void(int)> f(1);
  return 0;
}
