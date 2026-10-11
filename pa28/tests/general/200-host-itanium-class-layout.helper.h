// Class layouts both compilers must agree on (Itanium C++ ABI 2.4, SysV
// bit-fields): signature() is compiled into each translation unit.
struct Mixed { long a : 3; bool b : 1; long c : 20; };
struct Signs { unsigned v : 2; signed n : 3; bool f : 1; };
struct After { char c; int x : 12; char d; };
struct Base { Base(); int value; char tag; };
struct Derived : Base { char extra; short more; };
struct Empty {};
struct Mark { Mark(); char mark; };
struct Holder : Base, virtual Mark { Holder(); };
struct EmptyDerived : virtual Empty { EmptyDerived(); int tail; };

static int first_bit(const void* object, int size)
{
  const unsigned char* bytes = static_cast<const unsigned char*>(object);
  for (int bit = 0; bit < size * 8; ++bit)
    if (bytes[bit / 8] & (1 << (bit % 8))) return bit;
  return -1;
}

#define LAYOUT_BIT(Type, field, value) \
  { Type object; unsigned char* bytes = reinterpret_cast<unsigned char*>(&object); \
    for (unsigned i = 0; i < sizeof(Type); ++i) bytes[i] = 0; \
    object.field = value; signature = signature * 131 + first_bit(&object, sizeof(Type)); }
#define LAYOUT_OFFSET(object, part) \
  signature = signature * 131 + (reinterpret_cast<const char*>(part) - \
    reinterpret_cast<const char*>(&object));

static long signature()
{
  long signature = 0;
  signature = signature * 131 + sizeof(Mixed);
  signature = signature * 131 + sizeof(Signs);
  signature = signature * 131 + sizeof(After);
  signature = signature * 131 + sizeof(Derived);
  signature = signature * 131 + sizeof(Holder);
  signature = signature * 131 + sizeof(EmptyDerived);
  LAYOUT_BIT(Mixed, b, true)
  LAYOUT_BIT(Mixed, c, 1)
  LAYOUT_BIT(Signs, n, 1)
  LAYOUT_BIT(Signs, f, true)
  LAYOUT_BIT(After, x, 1)
  LAYOUT_BIT(After, d, 1)
  Derived derived;
  LAYOUT_OFFSET(derived, &derived.extra)
  LAYOUT_OFFSET(derived, &derived.more)
  Holder holder;
  LAYOUT_OFFSET(holder, static_cast<Base*>(&holder))
  LAYOUT_OFFSET(holder, static_cast<Mark*>(&holder))
  EmptyDerived empty;
  LAYOUT_OFFSET(empty, static_cast<Empty*>(&empty))
  LAYOUT_OFFSET(empty, &empty.tail)
  return signature;
}

long host_signature();
void host_fill(Mixed& mixed, Signs& signs, After& after);
int host_check(const Mixed& mixed, const Signs& signs, const After& after);
void host_assign_base(Base& target, const Base& source);
int host_read_mark(Holder& holder);
