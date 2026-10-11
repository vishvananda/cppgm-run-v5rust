template<class T>
struct Function;

template<class R, class... Args>
struct Function<R(Args...)> {
  Function() {}
  Function(const Function&);
};

template<class R, class... Args>
Function<R(Args...)>::Function(const Function&) {}

struct S {
  Function<void(int)> f;
};

int main() {
  return 0;
}
