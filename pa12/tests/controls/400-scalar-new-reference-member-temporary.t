// expect exit 0
// A reference member's temporary ends with the allocating full-expression.
int destroyed;
struct Temporary { ~Temporary() { ++destroyed; } };
struct Aggregate { const Temporary& reference; };
Aggregate* make() { return new Aggregate{Temporary()}; }
int main() {
  Aggregate* p = make();
  if (destroyed != 1) return 1;
  delete p;
  return destroyed == 1 ? 0 : 2;
}
