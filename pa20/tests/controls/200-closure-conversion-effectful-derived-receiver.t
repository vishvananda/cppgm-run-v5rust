// expect exit 0
// An inherited closure conversion evaluates its receiver once and retains
// conversion_object's base projection in the exact LowIR comparison.
using Pointer = int (*)();
auto closure = [] { static int count; return ++count; };
using Closure = decltype(closure);
struct Derived : Closure {
  Derived() : Closure(closure) {}
};
Derived object;
int effects;
Derived& reference() { ++effects; return object; }
Derived value() { ++effects; return object; }
int main()
{
  Pointer first = reference();
  Pointer second = value();
  Pointer third = static_cast<Derived&&>(reference());
  Pointer fourth = reference().operator Pointer();
  if (effects != 4) return 1;
  return first() == 1 && second() == 2 && third() == 3 && fourth() == 4 ? 0 : 2;
}
