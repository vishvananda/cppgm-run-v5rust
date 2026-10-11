// HHC-164
enum class E : unsigned long { A = 0 };

E f(unsigned long n) { return E(n); }
