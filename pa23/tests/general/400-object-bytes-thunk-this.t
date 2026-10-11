// lowir.md object_bytes and unwind=no: a thunk states its target's boundary,
// except that its `this` (pointing into the object at its adjustment) states no
// object_bytes; a deleting destructor's thunk states unwind=no as the deleting
// destructor does.
struct First { virtual long first() const { return 1; } virtual ~First() {} long a; };
struct Second { virtual long second() const { return 2; } virtual ~Second() {} long b; };
struct Joined : First, Second
{
  long second() const { return 3; }
  ~Joined() {}
  long c;
};

long ask(const Second& s) { return s.second(); }
void drop(Second* s) { delete s; }

int main()
{
  Joined* joined = new Joined;
  long result = ask(*joined);
  drop(joined);
  return result == 3 ? 0 : 1;
}
