struct Fun { void operator()() const {} };

template<class Sig>
struct F;

template<class R, class... Args>
struct F<R(Args...)> {
  F() {}

  template<class T>
  F& operator=(T) { return *this; }
};

int main() {
  Fun fun;
  F<void()> f;
  f = fun;
}
