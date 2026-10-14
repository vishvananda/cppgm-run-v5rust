// expect exit 0
// A reference binds the converted base pointer temporary, including constructor arguments.
struct Base { int value; };
struct Derived : Base { int extra; };
Base* saved;
void save(Base* const& pointer) { saved = pointer; }
struct Reader
{
  int value;
  Reader(Base* const& pointer) : value(pointer ? pointer->value : 0) {}
};
int main()
{
  Derived object;
  object.value = 9;
  Derived* pointer = &object;
  save(pointer);
  if (saved != &object) return 1;
  Base* const& reference = pointer;
  if (reference != &object) return 2;
  Reader reader(pointer);
  if (reader.value != 9) return 3;
  save(static_cast<Derived*>(0));
  if (saved != 0) return 4;
  return 0;
}
