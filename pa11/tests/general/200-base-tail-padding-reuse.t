// Itanium C++ ABI 2.4: a base that is not a POD for the purpose of layout
// claims only its non-virtual size, so the derived class's members reuse its
// tail padding; a POD base keeps its padding.
struct Base
{
  Base() : value(1), tag(2) {}
  int value;
  char tag;
};
struct Derived : Base
{
  Derived() : extra(3), more(4) {}
  char extra;
  short more;
};
struct Pod { int value; char tag; };
struct PodDerived : Pod { char extra; };

unsigned char byte_at(const void* object, int index)
{
  return static_cast<const unsigned char*>(object)[index];
}

int main()
{
  if (sizeof(Base) != 8 || sizeof(Derived) != 8 || sizeof(PodDerived) != 12)
    return 1;
  Derived derived;
  if (byte_at(&derived, 4) != 2 || byte_at(&derived, 5) != 3 ||
      byte_at(&derived, 6) != 4)
    return 2;
  Base& base = derived;
  base.tag = 9;
  base.value = 10;
  if (derived.extra != 3 || derived.more != 4 || derived.tag != 9 ||
      derived.value != 10)
    return 3;
  PodDerived pod;
  pod.tag = 5;
  pod.extra = 6;
  if (byte_at(&pod, 8) != 6 || pod.tag != 5) return 4;
  return 0;
}
