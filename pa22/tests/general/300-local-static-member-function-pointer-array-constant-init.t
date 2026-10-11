struct x { void f() {} };
typedef void (x::*member)();
member const * table() {
  static member const value[1] = { &x::f };
  return value;
}
int main() { return table() ? 0 : 1; }
