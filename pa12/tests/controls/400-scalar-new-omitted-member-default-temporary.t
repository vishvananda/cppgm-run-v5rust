// expect exit 0
// Default argument temporaries and storage cleanup have distinct lifetimes.
int destroyed;
struct Temporary { ~Temporary() { ++destroyed; } };
struct Member {
  Member(const Temporary& = Temporary()) {}
  ~Member() {}
};
struct Aggregate { int value; Member member; };
Aggregate* make() { return new Aggregate{}; }
int main() {
  Aggregate* p = make();
  if (destroyed != 1 || p->value) return 1;
  delete p;
  return destroyed == 1 ? 0 : 2;
}
