struct IOS
{
  int flags;
  IOS() : flags(0) {}
  void set(int v) { flags = v; }
  int get() const { return flags; }
};

struct OStream : virtual IOS
{
  OStream() {}
  void mark() { set(7); }
};

struct StringStream : OStream
{
  StringStream() {}
};

int main()
{
  StringStream ss;
  ss.mark();
  return ss.get() == 7 ? 0 : 1;
}
