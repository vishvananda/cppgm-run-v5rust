// expect exit 0
// N3485 12.2p3: the indirect call's class result belongs to this full-expression.
// A receiver with a trivial destructor can split its cleanup region before
// the conversion call; the result still reopens the region for destruction.
int destroyed;
int total;
struct Result {
  int value;
  Result(int n) : value(n) {}
  ~Result() { ++destroyed; total += value; }
};
Result target(int n) { return Result(n); }
typedef Result (*Pointer)(int);
struct Callable { operator Pointer() { return target; } };
struct Fallback {
  int operator()(char*);
  operator Pointer() { return target; }
};
void use() {
  Callable()(7);
  Fallback()(9);
}
int main() {
  use();
  return destroyed == 2 && total == 16 ? 0 : 1;
}
