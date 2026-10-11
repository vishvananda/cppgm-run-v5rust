struct X {
private:
  int v;
public:
  X(int x) : v(x) {}
  friend bool eq(const X& a, const X& b);
};

bool eq(const X& a, const X& b) {
  return a.v == b.v;
}

int main() {
  return eq(X(1), X(1)) ? 0 : 1;
}
