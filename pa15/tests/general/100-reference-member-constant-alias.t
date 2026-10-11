template<long N>
struct ratio1 {
  static const long num = N;
  typedef ratio1<num> type;
};

template<class Period>
struct duration1 {
  static const long value = Period::num;
};

typedef duration1<ratio1<7> > X;

int main() { return 0; }
