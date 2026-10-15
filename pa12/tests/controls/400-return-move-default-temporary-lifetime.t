// N3485 12.8p32, 12.2p3: return move defaults end after the transfer; expect exit 0
int defaults;
int alive;
int moves;
int bad;
struct Default
{
  Default() { ++defaults; }
  ~Default() { --defaults; }
};
struct Value
{
  int n;
  Value(int v = 0) : n(v) { ++alive; }
  Value(const Value& v) : n(v.n) { ++alive; }
  Value(Value&& v, const Default& = Default()) : n(v.n)
  {
    ++alive;
    ++moves;
    if (defaults != 1) ++bad;
    v.n = 0;
  }
  ~Value() { --alive; }
};
Value result(Value value) { return value; }
int main()
{
  {
    Value a(19);
    Value b = result(a);
    if (a.n != 19 || b.n != 19 || defaults != 0 || alive != 2) ++bad;
  }
  return bad || alive != 0 || defaults != 0 || moves != 1;
}
