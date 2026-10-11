// A reference or pointer reaches an indirect virtual base through that base's
// own vbase-offset row: where another virtual base sits relative to it
// depends on the complete object, so no static second hop is valid.
struct IOS { int value; };
struct BasicIOS : virtual IOS { int pad; };
struct IStream : virtual BasicIOS {};
struct OStream : virtual BasicIOS {};
struct IOStream : IStream, OStream {};
struct Other : virtual IOS { int other[5]; };
struct Weird : Other, OStream {};

struct B { int value; B() : value(7) {} };
struct H : virtual B {};
struct E : virtual H {};
struct Z : virtual B { int z[4]; };
struct F : Z, E {};

int read(OStream* out) { return out->value; }
const B* convert(const E& e) { return &e; }

int main()
{
  IOStream stream;
  stream.value = 7;
  if (read(&stream) != 7) return 1;
  Weird weird;
  weird.value = 9;
  if (read(&weird) != 9) return 2;
  F f;
  f.value = 9;
  if (convert(f) != static_cast<const B*>(&f)) return 3;
  E e;
  if (convert(e) != static_cast<const B*>(&e) || convert(e)->value != 7) return 4;
  return 0;
}
