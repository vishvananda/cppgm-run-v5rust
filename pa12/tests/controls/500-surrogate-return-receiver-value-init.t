// expect exit 0
// N3485 8.5p8, 12.2p3: a surrogate's receiver retains its own value
// initialization when the indirect class result uses the return destination.
int destroyed;
struct Result {
  int value;
  Result(int n) : value(n) {}
  ~Result() { ++destroyed; }
};
Result target(int n) { return Result(n); }
typedef Result (*Pointer)(int);
struct Callable { operator Pointer() { return target; } };
Result forward(int n) { return Callable()(n); }
int main() {
  { Result result = forward(7); if (result.value != 7) return 1; }
  return destroyed == 1 ? 0 : 2;
}
