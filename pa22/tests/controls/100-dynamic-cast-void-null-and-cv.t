// expect exit 0
// Most-derived void casts preserve null, cv-qualification and operand effects.
struct Base { virtual ~Base() {} };
struct Derived : Base { int value; Derived() : value(42) {} };
int evaluations;
Base* observe(Base* object) { ++evaluations; return object; }
const Base* observe_const(const Base* object) { ++evaluations; return object; }
volatile Base* observe_volatile(volatile Base* object) { ++evaluations; return object; }
int main() {
  Derived object;
  Base* base = &object;
  if (dynamic_cast<void*>(observe(base)) != &object || evaluations != 1) return 1;
  if (dynamic_cast<void*>(observe(0)) != 0 || evaluations != 2) return 2;
  if (dynamic_cast<const void*>(observe_const(base)) != &object || evaluations != 3) return 3;
  if (dynamic_cast<const void*>(observe_const(0)) != 0 || evaluations != 4) return 4;
  if (dynamic_cast<volatile void*>(observe_volatile(base)) != &object || evaluations != 5) return 5;
  if (dynamic_cast<volatile void*>(observe_volatile(0)) != 0 || evaluations != 6) return 6;
  return 0;
}
