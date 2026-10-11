template<long long N, long long D = 1>
struct ratio {
  static const long long num = N;
  static const long long den = D;
  typedef ratio<num, den> type;
};

ratio<1, 2>::type *p;

int main() {
  return 0;
}
