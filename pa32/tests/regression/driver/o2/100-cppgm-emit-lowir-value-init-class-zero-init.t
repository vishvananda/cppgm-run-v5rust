struct D { D() {} };
struct S { int type; long long value; D d; };

int f() {
  S s;
  s = S();
  return s.type + (int)s.value;
}

int main() {
  return f();
}
