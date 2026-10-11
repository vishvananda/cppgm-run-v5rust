// N3485 11.3p11: friend class Y in a local class names the Y of the innermost
// enclosing block; expect exit 0
int main() {
  class L1 { friend class Y; int a = 1; };
  class Y { public: int f(L1 x) { return x.a; } };
  return Y().f(L1()) == 1 ? 0 : 1;
}
