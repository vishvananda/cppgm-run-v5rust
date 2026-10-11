// Itanium C++ ABI 2.4 III: virtual bases are placed after the non-virtual
// part from its data size, reusing a non-POD base's tail padding, and an
// empty virtual base at offset zero when no subobject of its type is there.
// Copying or value-initializing the non-virtual base afterwards must not
// write the virtual base that lives in its tail padding.
struct EmptyRoot {};
struct EmptyDerived : virtual EmptyRoot {};
struct Tail
{
  Tail() : value(1), tag(2) {}
  int value;
  char tag;
};
struct Mark
{
  Mark() : mark(5) {}
  char mark;
};
struct Holder : Tail, virtual Mark
{
  Holder() {}
  Holder(const Holder& other) : Mark(other), Tail(other) {}
};
struct Plain
{
  int value;
  char tag;
private:
  char hidden;
};
struct Zeroed : Plain, virtual Mark
{
  Zeroed() : Plain() {}
};

long offset_of(const void* base, const void* object)
{
  return static_cast<const char*>(base) - static_cast<const char*>(object);
}

int main()
{
  EmptyDerived empty;
  if (sizeof(EmptyDerived) != 8 ||
      offset_of(static_cast<EmptyRoot*>(&empty), &empty) != 0)
    return 1;
  Holder holder;
  if (sizeof(Holder) != 16 ||
      offset_of(static_cast<Mark*>(&holder), &holder) != 13)
    return 2;
  holder.mark = 6;
  holder.tag = 7;
  Holder copy(holder);
  if (copy.mark != 6 || copy.tag != 7 || copy.value != 1) return 3;
  Zeroed zeroed;
  if (sizeof(Zeroed) != 16 || zeroed.mark != 5 || zeroed.value != 0 ||
      zeroed.tag != 0)
    return 4;
  return 0;
}
