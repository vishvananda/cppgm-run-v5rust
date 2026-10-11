template<long N, long D = 1>
struct Ratio {
  static const long num = N;
  static const long den = D;
  typedef Ratio<num, den> type;
};

template<long A, long B>
struct SafeMultiply {
  static const long value = A * B;
};

template<class R1, class R2>
struct RatioMultiply {
  static const long gcd1 = 1;
  static const long gcd2 = 1;

  typedef Ratio<
      SafeMultiply<(R1::num / gcd1), (R2::num / gcd2)>::value,
      SafeMultiply<(R1::den / gcd2), (R2::den / gcd1)>::value> type;
};

template<class R1, class R2>
struct RatioDivide {
  typedef typename RatioMultiply<R1, Ratio<R2::den, R2::num> >::type type;
};

template<class R1, class R2>
using ratio_divide = typename RatioDivide<R1, R2>::type;

template<class Rep, class Period = Ratio<1> >
struct Duration {
  typedef Rep rep;
  typedef Period period;
};

template<class ToDuration, class Rep, class Period>
long duration_cast(Duration<Rep, Period>)
{
  typedef typename ToDuration::period to_period;
  typedef ratio_divide<Period, to_period> cf;
  return cf::num;
}

int main()
{
  Duration<long, Ratio<1, 1000> > from;
  return duration_cast<Duration<long, Ratio<1, 1> > >(from) == 1000 ? 0 : 1;
}
