// expect exit 0
int calls;
struct Base { int value; };
struct Derived : Base {};
struct Source
{
  Derived * object;
  operator Derived &() & { ++calls; return *object; }
  operator Derived &&() && { ++calls; return static_cast<Derived &&>(*object); }
};
struct Inherited : Source {};
int main()
{
  Derived object;
  object.value = 7;
  Base other;
  other.value = 3;
  Inherited source;
  source.object = &object;
  Base & left = true ? ((source)) : other;
  Base & right = false ? source : other;
  if (&left != static_cast<Base *>(&object) || &right != &other || calls != 1)
    return 1;
  Base && moved = true ? static_cast<Source &&>(source) : static_cast<Base &&>(other);
  if (&moved != static_cast<Base *>(&object) || calls != 2) return 2;
  moved.value = 9;
  return object.value != 9;
}
