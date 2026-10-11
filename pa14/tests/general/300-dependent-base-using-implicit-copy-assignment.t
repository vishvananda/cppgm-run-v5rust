// VALIDATION: compile-pass
// N3485 focus: 12.8 [class.copy], 14.6.2 [temp.dep]
// A using-declaration in a class template may import an implicitly declared
// assignment operator from its concrete dependent base.

struct slot
{
  int value;
};

template<class Slot>
struct filtering_slot : Slot
{
  using Slot::operator=;
};

int main()
{
  slot source;
  source.value = 7;

  filtering_slot<slot> target;
  target.value = 0;
  target = source;
  return target.value == 7 ? 0 : 1;
}
