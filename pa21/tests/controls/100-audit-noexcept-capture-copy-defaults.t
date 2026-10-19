// expect exit 0
int tokens, live;
struct Token { Token() { ++tokens; } ~Token() { --tokens; } };
struct Item {
  int n;
  Item() : n(3) { ++live; }
  Item(const Item& x, Token = Token()) : n(x.n) { ++live; }
  virtual ~Item() { --live; }
};
int small() noexcept {
  Item values[2];
  auto call = [=]() { return values[0].n + values[1].n; };
  return call();
}
int large() noexcept {
  Item values[9];
  auto call = [=]() { return values[8].n; };
  return call();
}
int outside() noexcept {
  Item values[2];
  try { auto call = [values]() { return values[0].n; }; return call(); }
  catch (...) { return 0; }
}
int main() { return small() == 6 && large() == 3 && outside() == 3 && tokens == 0 && live == 0 ? 0 : 1; }
