template<long long V>
struct Box {};

template<long long X>
struct Limits {
  static const long long nan = (1LL << (sizeof(long long) * 8 - 1));
  static const long long min = nan + 1;
  static const long long max = -min;
  typedef Box<max> type;
};

int main() {
  Limits<0>::type value;
  (void)value;
  return 0;
}
