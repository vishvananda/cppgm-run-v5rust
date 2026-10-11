struct S { int value; };
struct B1 { virtual void anchor() {} };
struct B2 { virtual S current() const = 0; };
struct D : B1, B2 {
  S current() const { S out = {7}; return out; }
};
int main() { D d; B2 & b = d; return b.current().value - 7; }
