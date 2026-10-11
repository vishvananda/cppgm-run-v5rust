template<class> struct c {
  template<class> struct i {
    template<class U> i(i<U> const &) {}
  };
};

using C = c<int>;
C::i<long> f(C::i<int> value) { return value; }

int main() {}
