namespace N {
namespace _Algorithm {
struct __copy {};
}

template<bool Cond, class T = int>
struct EnableIf {};

template<class T>
struct EnableIf<true, T> {
  typedef T type;
};

template<class AlgTag, class A, class B>
struct specialized_algorithm {
  static const bool has = false;
};

struct __copy_impl {
  template<class In, class Sent, class Out,
           typename EnableIf<!specialized_algorithm<_Algorithm::__copy, In, Out>::has, int>::type = 0>
  int operator()(In, Sent, Out) const { return 7; }
};

template<class _Algorithm, class In, class Sent, class Out>
int copy_move_unwrap_iters(In first, Sent last, Out out) {
  return _Algorithm()(first, last, out);
}
}

int main() {
  return N::copy_move_unwrap_iters<N::__copy_impl>(1, 2, 3) == 7 ? 0 : 1;
}
