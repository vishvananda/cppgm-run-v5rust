template<class T, T V> struct integral_constant {};

template<long long N, long long D = 1> struct ratio {
  static const long long num = N;
  static const long long den = D;
};

template<long long N, long long D>
const long long ratio<N, D>::num;

template<class R1, class R2, bool B1 = true, bool B2 = false>
struct ratio_less;

template<class R1, class R2>
struct ratio_less<R1, R2, true, false>
  : integral_constant<bool, R1::num < R2::num> {};
