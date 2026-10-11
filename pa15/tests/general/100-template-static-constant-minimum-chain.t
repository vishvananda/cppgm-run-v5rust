template<long long X>
struct Limits {
  static const long long nan = (1LL << (sizeof(long long) * 8 - 1));
  static const long long min = nan + 1;
  static const long long max = -min;
  static_assert(min < 0, "min");
  static_assert(max > 0, "max");
};

int main() {
  Limits<0> value;
  (void)value;
  return 0;
}
