template<class T, T min_val>
struct base { static const T const_min = min_val; };

template<class T, T min_val>
const T base<T, min_val>::const_min;

struct traits : base<int, -1000> {};
struct uchar_traits : base<unsigned char, 255> {};

template<int N>
struct box { static const int value = N; };

template<int MinValue>
struct use {
  typedef box<(MinValue >= traits::const_min)> type;
};

static_assert(use<-200>::type::value == 1, "");

template<long long MinValue>
struct use_signed_min {
  typedef box<(MinValue >= (-9223372036854775807LL - 1LL))> type;
};

static_assert(use_signed_min<(-9223372036854775807LL - 1LL)>::type::value == 1, "");

template<unsigned long long MaxValue>
struct use_unsigned_max {
  typedef box<(MaxValue <= uchar_traits::const_min)> type;
};

static_assert(use_unsigned_max<18446744073709551615ULL>::type::value == 0, "");

int main() { return 0; }
