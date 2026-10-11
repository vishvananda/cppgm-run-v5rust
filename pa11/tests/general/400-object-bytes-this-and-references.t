// lowir.md object_bytes: a pointer parameter states object_bytes=N, N the size
// of the object type, when it designates the start of an object of that
// complete type: `this` of every member function, constructor and destructor,
// a reference to a complete object type (also the class aggregate helper's
// `this` and reference members); not a reference to an incomplete class or a
// function, nor a plain pointer.
struct Opaque;
struct Triple
{
  long first, second, third;
  Triple(long base) : first(base), second(base + 1), third(base + 2) {}
  ~Triple() {}
  long total() const { return first + second + third; }
};
struct Holder { int& target; long tag; };

long by_reference(const Triple& value) { return value.total(); }
long by_scalar_reference(const long& value) { return value; }
long by_pointer(const Triple* value) { return value->first; }
int by_opaque(Opaque& value) { return &value != 0; }
int call_through(int (&function)(int), int x) { return function(x); }
int twice(int x) { return 2 * x; }

int main()
{
  Triple value(5);
  int target = 4;
  Holder holders[1] = {{target, 6}};
  return by_reference(value) != 18 || by_scalar_reference(value.second) != 6 ||
    by_pointer(&value) != 5 || call_through(twice, 3) != 6 || holders[0].target != 4;
}
