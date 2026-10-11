struct X {
  int v;
  friend bool operator==(const X& a, const X& b) { return a.v == b.v; }
  friend bool operator!=(const X& a, const X& b) { return !(a == b); }
};

int main() { return 0; }
