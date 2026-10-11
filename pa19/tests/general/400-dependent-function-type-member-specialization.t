template <class Sig> struct Value;
template <class R, class... Args>
struct Value<R(Args...)> {
  int x;
  Value() : x(7) {}
};

template <class Sig> struct Function;
template <class R, class... Args>
struct Function<R(Args...)> {
  Value<R(Args...)> f;
  int get() const { return f.x; }
};

int main() {
  return Function<void()>().get() - 7;
}
