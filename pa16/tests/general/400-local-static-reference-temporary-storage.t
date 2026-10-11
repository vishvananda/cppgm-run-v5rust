// N3485 12.2p5, 3.7.1: a temporary bound to a block-scope static reference
// persists for the lifetime of the reference, so it has static storage
// duration rather than the frame of the call that initializes it.
int calls;
struct Value
{
  int tag;
  long payload;
  Value(int n) : tag(n), payload(100 + n) {}
};
int next() { return ++calls + 40; }
bool first() { return calls == 0; }

const Value& whole() { static const Value& held = Value(1); return held; }
const long& member() { static const long& held = Value(2).payload; return held; }
const Value& chosen() { static const Value& held = first() ? Value(3) : Value(4); return held; }
const int& scalar() { static const int& held = next(); return held; }

int clobber()
{
  volatile long frame[16];
  for (int i = 0; i < 16; ++i) frame[i] = -1;
  return static_cast<int>(frame[5]);
}

int main()
{
  const Value& a = whole();
  clobber();
  const long& b = member();
  clobber();
  const Value& c = chosen();
  clobber();
  const int& d = scalar();
  clobber();
  return a.payload == 101 && &a == &whole() && b == 102 && &b == &member() &&
    c.payload == 103 && &c == &chosen() && d == 41 && &d == &scalar() &&
    calls == 1 ? 0 : 1;
}
