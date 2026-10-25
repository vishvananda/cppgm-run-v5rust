// N3485 1.9p10, 12.2p3: a parenthesized initializer's arguments form
// one full-expression. A throwing destination constructor does not make
// its nonthrowing argument calls throw during the segment query.
// expect exit 0
int alive;
int destroyed;
struct Argument {
  Argument() { ++alive; }
  Argument(const Argument&) noexcept { ++alive; }
  ~Argument() noexcept { --alive; ++destroyed; }
};
Argument make_argument() noexcept { return Argument(); }
struct Destination {
  int sum;
  Destination(int first, int second, const Argument&) : sum(first + second) {}
};
struct ValueDestination {
  ValueDestination(Argument) noexcept {}
};
int run(int first, int second) {
  Destination value(first, second, make_argument());
  ValueDestination single(make_argument());
  return value.sum;
}
int main() {
  return run(2, 3) != 5 || alive != 0 || destroyed != 2;
}
