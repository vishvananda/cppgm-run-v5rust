// lowir.md object_bytes: a class object passed by address (pass=by_address) and
// an indirect result (pass=indirect_result) state their class's size.
struct Large
{
  long a, b, c;
  Large(long x) : a(x), b(x + 1), c(x + 2) {}
  Large(const Large& other) : a(other.a), b(other.b), c(other.c) {}
};

long consume(Large value) { return value.a + value.b + value.c; }
Large produce(long x) { return Large(x); }

int main()
{
  return consume(produce(1)) == 6 ? 0 : 1;
}
