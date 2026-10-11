template<long N, long D = 1>
struct ratio {
  static const long num = N;
  static const long den = D;
  typedef ratio<num, den> type;
};

template<class Period>
struct duration {
  static const long value = Period::num;
  typedef typename Period::type canonical;
};

typedef duration<ratio<60> > minutes;

int main() { return 0; }
