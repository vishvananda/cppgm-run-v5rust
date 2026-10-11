// A nearly empty virtual base that is the primary base of several base
// subobjects lies with the first of them in inheritance graph order (Itanium
// 2.4; g++'s mark_primary_bases): Y, a virtual base, before X, a non-virtual
// one placed first.  The class's own virtual primary base's empty
// subobjects take part in placing later empty bases (EmptyBase in Base and
// in Second cannot share an offset).
struct A { virtual int f(); };
struct X : virtual A { int x; };
struct Y : virtual A { int y; };
struct Z : virtual Y, X { int z; };
struct EmptyBase {};
struct Second : EmptyBase {};
struct Base : EmptyBase { virtual int b(); };
struct Holder : virtual Base, virtual Second { int h; };

#define LAYOUT_OFFSET(object, part) \
  signature = signature * 131 + (reinterpret_cast<const char*>(part) - \
    reinterpret_cast<const char*>(&object));

static long signature()
{
  long signature = 0;
  signature = signature * 131 + sizeof(Z);
  signature = signature * 131 + sizeof(Holder);
  Z z;
  LAYOUT_OFFSET(z, static_cast<A*>(&z))
  LAYOUT_OFFSET(z, static_cast<Y*>(&z))
  LAYOUT_OFFSET(z, static_cast<X*>(&z))
  LAYOUT_OFFSET(z, &z.z)
  Holder holder;
  LAYOUT_OFFSET(holder, static_cast<Base*>(&holder))
  LAYOUT_OFFSET(holder, static_cast<Second*>(&holder))
  LAYOUT_OFFSET(holder, &holder.h)
  return signature;
}

long host_signature();
int host_f(A& value);
int host_y(Y& value);
Z* host_make();
