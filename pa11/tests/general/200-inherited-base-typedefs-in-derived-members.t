struct Base {
  typedef const char* P;
};

struct X : Base {
  typedef P Q;
  P value;
  static int f(P) noexcept { return 0; }
};

int main() { return 0; }
