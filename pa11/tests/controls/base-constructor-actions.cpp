// N3485 12.6.2p10: bases precede members regardless of mem-initializer order;
// an aliased protected base constructor retains reference/default arguments.
// expect exit 0
int sequence;
int next() { return ++sequence; }

struct Base {
  int first;
  const int& source;
protected:
  Base(int value, const int& input, int extra = 3)
    : first(value + extra), source(input) {}
};
typedef Base Alias;
struct Derived : Base {
  int last;
  Derived(int& input) : last(next()), Alias(next(), input) {}
};

struct Initialized { int value = 7; };
struct Middle : Initialized {};
struct Leaf : Middle {};
struct ValueBase { int value; ValueBase() = default; };
struct ValueDerived : ValueBase {
  int after;
  ValueDerived() : ValueBase(), after(9) {}
};

int main() {
  int input = 4;
  Derived object(input);
  if (object.first != 4 || object.last != 2 || object.source != 4) return 1;
  input = 8;
  if (object.source != 8) return 2;
  Initialized base;
  Leaf leaf;
  if (leaf.value != 7 || base.value != 7) return 3;
  ValueDerived zero;
  return zero.value == 0 && zero.after == 9 ? 0 : 4;
}
