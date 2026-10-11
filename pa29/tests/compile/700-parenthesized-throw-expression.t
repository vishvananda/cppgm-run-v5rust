#define THROW_WRAP(x) (throw (x))

struct E {};

inline void f() { THROW_WRAP(E()); }

int main() {
  return 0;
}
