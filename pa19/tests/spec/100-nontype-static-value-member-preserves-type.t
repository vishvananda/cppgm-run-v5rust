// VALIDATION: compile-pass
// N3485 focus: 14.3.2 [temp.arg.nontype], 7.1.5 [dcl.constexpr]

template<long V>
struct static_id {
  static constexpr long value = V;
};

template<long V>
struct static_abs {
  static constexpr long value = V < 0 ? -V : V;
};

template<long A, long B>
struct static_gcd {
  static constexpr long value = static_gcd<B, A % B>::value;
};

template<long A>
struct static_gcd<A, 0> {
  static constexpr long value = static_abs<A>::value;
};

template<long N, long D = 1>
struct ratio {
  static constexpr long num = N / static_gcd<N, D>::value;
  static constexpr long den = static_abs<D>::value / static_gcd<N, D>::value;
  typedef ratio<num, den> type;
};

template<class Rep, class Period>
struct duration {
  typedef Period period;
};

typedef duration<long, ratio<1, 1000000000> > nanos;
typedef nanos::period nanos_period;
typedef typename nanos_period::type normalized_period;

static_assert(static_id<42>::value == 42, "static value member preserves type");
static_assert(static_abs<1000000000>::value == 1000000000, "absolute value preserved");
static_assert(static_gcd<1, 1000000000>::value == 1, "gcd preserved");
static_assert(nanos_period::den == 1000000000, "duration period preserved");
static_assert(normalized_period::den == 1000000000, "ratio type preserves den");

int main()
{
  return 0;
}
