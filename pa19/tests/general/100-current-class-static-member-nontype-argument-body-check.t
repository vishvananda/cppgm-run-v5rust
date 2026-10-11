// N3485 focus: [temp.dep.expr] value-dependent expressions in template bodies.
template<unsigned long long A,
         unsigned long long B,
         unsigned long long C,
         unsigned long long D>
struct helper {
  typedef unsigned long long result_type;
  static result_type next(result_type x) { return x + D; }
};

template<class T, T A, T B, T C>
struct engine {
  typedef T result_type;
  static const result_type M = result_type(-1);
  result_type x;
  result_type run() { return helper<A, B, C, M>::next(x); }
};

int main() {
  engine<unsigned, 1, 0, 2> e;
  e.x = 3;
  return e.run() == 2 ? 0 : 1;
}
