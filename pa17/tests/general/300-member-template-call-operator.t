// HHC-047
struct X {
  template<class T1, class T2>
  bool operator()(const T1&, const T2&) const { return true; }
};

int main() { return 0; }
