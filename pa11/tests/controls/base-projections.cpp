// expect exit 0
// Field, receiver, pointer/reference and cast projections share retained base paths.
struct Base
{
  int value;
  int read() const { return value; }
};
struct Middle : Base {};
struct Derived : Middle
{
  int extra;
  int sum() const { return read() + value + extra; }
};
Base* as_pointer(Derived* object) { return object; }
Base& as_reference(Derived& object) { return object; }
Base& cast_reference(Derived& object) { return static_cast<Base&>(object); }
Derived* cast_pointer(Base* object) { return static_cast<Derived*>(object); }
class Reexposed : private Base
{
public:
  using Base::value;
  using Base::read;
};

int main()
{
  Derived object;
  object.value = 7;
  object.extra = 3;
  Base* pointer = as_pointer(&object);
  Base& reference = as_reference(object);
  if (pointer->read() != 7 || object.sum() != 17) return 1;
  reference.value = 9;
  if (cast_reference(object).value != 9 || cast_pointer(pointer)->extra != 3) return 2;
  if (as_pointer(0) != 0 || cast_pointer(0) != 0) return 3;
  Reexposed reexposed;
  reexposed.value = 11;
  if (reexposed.read() != 11) return 4;
  return 0;
}
