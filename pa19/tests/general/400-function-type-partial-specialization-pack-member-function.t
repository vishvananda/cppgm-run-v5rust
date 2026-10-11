template<class Sig>
class Base;

template<class R, class... Args>
class Base<R(Args...)> {
public:
  R call(Args&&...);
};

template<class Sig>
class Value;

template<class R, class... Args>
class Value<R(Args...)> {
  typedef Base<R(Args...)> Fun;
  Fun* f;
public:
  Value(const Value& x);
};

template<class R, class... Args>
Value<R(Args...)>::Value(const Value& x) : f(x.f) {}

int main() {
  return 0;
}
