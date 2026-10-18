// expect exit 0
// Implicit and explicit conversions evaluate their receivers once and share
// the call operator's local static with its static invoker.
using Pointer = int (*)();
int effects;
int main()
{
  auto next = []() -> int { static int count = 0; return ++count; };
  Pointer p = (++effects, next);
  Pointer q = (++effects, next).operator Pointer();
  if (effects != 2) return 1;
  return next() == 1 && p() == 2 && q() == 3 ? 0 : 2;
}
