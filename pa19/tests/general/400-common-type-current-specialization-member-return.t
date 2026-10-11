template<long long N, long long D = 1>
struct ratio {
  typedef ratio type;
  static const long long num = N;
  static const long long den = D;
};

template<class P1, class P2>
struct __ratio_gcd {
  typedef P1 type;
};

template<class...> struct common_type;

template<class T>
struct common_type<T> : common_type<T, T> {};

template<class T, class U>
struct common_type<T, U> {
  typedef T type;
};

template<class Rep, class Period>
struct duration;

template<class Rep1, class Period1, class Rep2, class Period2>
struct common_type<duration<Rep1, Period1>, duration<Rep2, Period2> > {
  typedef duration<typename common_type<Rep1, Rep2>::type,
                   typename __ratio_gcd<Period1, Period2>::type> type;
};

template<class Rep, class Period>
struct duration {
  typename common_type<duration>::type operator+() const {
    return typename common_type<duration>::type(*this);
  }
};

typedef duration<long long, ratio<1, 1000000000> > nanoseconds;
nanoseconds x;

int main() {
  return 0;
}
