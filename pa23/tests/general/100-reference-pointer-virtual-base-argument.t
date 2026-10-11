struct IOS {
  int value;
};

struct BasicIOS : virtual IOS {};
struct IStream : virtual BasicIOS {};
struct OStream : virtual BasicIOS {};
struct IOStream : IStream, OStream {};

struct StringStream : IOStream {
  StringStream()
  {
    value = 7;
  }
};

int read(OStream* out)
{
  return out->value;
}

int call(StringStream* const& ptr)
{
  return read(ptr);
}

int main()
{
  StringStream stream;
  StringStream* ptr = &stream;
  return call(ptr) == 7 ? 0 : 1;
}
