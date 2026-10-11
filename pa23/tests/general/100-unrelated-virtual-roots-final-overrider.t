// VALIDATION: compile-pass
struct Abstract { virtual void f() = 0; };
struct Derived1 : Abstract { void f() override {} };
struct Abstract2 { virtual void f() = 0; };
struct Derived2 : Abstract2 { void f() override {} };
struct Combined : Derived1, Derived2 {};
int main() { Combined combined; static_cast<Derived1*>(&combined)->f();
  static_cast<Derived2*>(&combined)->f(); }
