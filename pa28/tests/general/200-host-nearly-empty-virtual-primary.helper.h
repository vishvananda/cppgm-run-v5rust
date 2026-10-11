// A nearly empty virtual base is the primary base of a class with no
// non-virtual dynamic base (Itanium 2.4 I.b): it sits at offset zero and
// shares the class's vptr and vtable, whose prefix carries its vcall
// offsets before its vbase offset; an indirect primary base lies with the
// base that claims it.
struct Interface { virtual int id() = 0; virtual ~Interface() {} };
struct LeftImpl : virtual Interface { int id(); };
struct RightImpl : virtual Interface {};
struct Joined : LeftImpl, RightImpl { int data; int id(); };
struct Tag { virtual int tag() { return 1; } };
struct Tagged : virtual Tag { int value; Tagged(); int tag(); };
struct MoreTagged : Tagged { int more; MoreTagged(); };

#define LAYOUT_OFFSET(object, part) \
  signature = signature * 131 + (reinterpret_cast<const char*>(part) - \
    reinterpret_cast<const char*>(&object));

static long signature()
{
  long signature = 0;
  signature = signature * 131 + sizeof(LeftImpl);
  signature = signature * 131 + sizeof(Joined);
  signature = signature * 131 + sizeof(Tagged);
  signature = signature * 131 + sizeof(MoreTagged);
  Joined joined;
  LAYOUT_OFFSET(joined, static_cast<Interface*>(&joined))
  LAYOUT_OFFSET(joined, static_cast<RightImpl*>(&joined))
  LAYOUT_OFFSET(joined, &joined.data)
  MoreTagged more;
  LAYOUT_OFFSET(more, static_cast<Tag*>(&more))
  LAYOUT_OFFSET(more, &more.value)
  LAYOUT_OFFSET(more, &more.more)
  return signature;
}

long host_signature();
int host_id(Interface& value);
int host_right_id(RightImpl& value);
int host_tag(Tag& value);
MoreTagged* host_make();
