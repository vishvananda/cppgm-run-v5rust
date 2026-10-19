// expect exit 0
int events[8], count, live;
bool recording;
void event(int n) { if (recording && count < 8) events[count++] = n; }
struct Token {
  Token() { ++live; }
  Token(const Token&) { ++live; }
  ~Token() { --live; event(2); }
};
struct Item {
  int n;
  Item() : n(7) {}
  Item(const Item& x, Token = Token()) : n(x.n) {}
  virtual ~Item() { event(1); }
};
int run() {
  Item original;
  auto call = [&]() {
    try { throw original; }
    catch (Item caught) { recording = true; return caught.n; }
  };
  return call();
}
int main() {
  int value = run();
  return value == 7 && live == 0 && count >= 2 && events[0] == 1 && events[1] == 2 ? 0 : 1;
}
