namespace N {
struct X;
int eq(const X&, const X&);

struct X {
private:
  int v;
public:
  X(int x) : v(x) {}
  friend int N::eq(const X&, const X&);
};

int eq(const X& a, const X& b) {
  return a.v == b.v;
}
}

int main() {
  return N::eq(N::X(1), N::X(1)) ? 0 : 1;
}
