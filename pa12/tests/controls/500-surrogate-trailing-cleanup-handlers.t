// expect exit 0
// N3485 12.2p3, 15.2: a throwing receiver destructor closes the result's
// empty region. Its already allocated cleanup handler remains in the body.
int destroyed;
struct Result {
  int value;
  Result(int n) : value(n) {}
  ~Result() { destroyed = destroyed * 10 + value; }
};
Result target(int n) { return Result(n); }
typedef Result (*Pointer)(int);
struct Callable {
  Callable() noexcept {}
  ~Callable() noexcept(false) { destroyed = destroyed * 10 + 2; }
  operator Pointer() { return target; }
};
void use() { Callable()(1); }
int main() {
  use();
  return destroyed == 12 ? 0 : 1;
}
