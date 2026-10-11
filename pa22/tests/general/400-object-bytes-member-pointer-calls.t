// lowir.md object_bytes on call signatures: the object of a call through a
// pointer to member function of class C states sizeof(C) when C has no base
// class; when C has one, the pointer may adjust the object to a base subobject
// (5.5p6) and it states none. A noexcept pointer to member's call states
// unwind=no.
struct Left { long left; long get_left() const { return left; } };
struct Right { long right; long get_right() const { return right; } };
struct Both : Left, Right { long both; };
struct Alone { long value; long get() const noexcept { return value; } };

long call_both(const Both& object, long (Both::*member)() const) { return (object.*member)(); }
long call_alone(const Alone& object, long (Alone::*member)() const noexcept) { return (object.*member)(); }

int main()
{
  Both both;
  both.left = 1;
  both.right = 2;
  both.both = 3;
  Alone alone;
  alone.value = 4;
  return call_both(both, &Both::get_left) + call_both(both, &Both::get_right) +
    call_alone(alone, &Alone::get) == 7 ? 0 : 1;
}
