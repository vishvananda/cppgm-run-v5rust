struct X {
private:
  int v;
public:
  X(int x) : v(x) {}
  friend bool operator==(const X& a, const X& b) { return a.v == b.v; }
  friend bool operator!=(const X& a, const X& b) { return !(a == b); }
};

int main() {
  return X(1) != X(2) ? 0 : 1;
}
