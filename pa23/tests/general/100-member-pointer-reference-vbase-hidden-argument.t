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
};

struct StringStream : OStream
{
  StringStream() {}
};

struct Writer
{
  void mark(OStream & os) const
  {
    os.set(9);
  }
};

int main()
{
  StringStream ss;
  Writer writer;
  void (Writer::*member)(OStream &) const = &Writer::mark;
  (writer.*member)(ss);
  return ss.get() == 9 ? 0 : 1;
}
