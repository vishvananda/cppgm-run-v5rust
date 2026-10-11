// SysV/Itanium bit-field allocation: adjacent bit-fields of different
// declared types share storage, a field moves to the next unit of its own
// type only rather than straddle one, a zero-width field moves to its type's
// alignment, and an unnamed field does not align the class.
struct Mixed { long a : 3; bool b : 1; };
struct Signs { unsigned v : 2; signed n : 3; bool f : 1; };
struct After { char c; int x : 4; };
struct Before { int x : 4; char c; };
struct Straddle { char c; int x : 20; char d; };
struct Zero { unsigned char a : 3; long : 0; unsigned char b : 3; };
struct Unnamed { char c; long : 3; char d; };

unsigned char byte_at(const void* object, int index)
{
  return static_cast<const unsigned char*>(object)[index];
}

int main()
{
  if (sizeof(Mixed) != 8 || sizeof(Signs) != 4 || sizeof(After) != 4 ||
      sizeof(Before) != 4 || sizeof(Straddle) != 8 || sizeof(Zero) != 9 ||
      sizeof(Unnamed) != 3)
    return 1;
  Mixed mixed = {0, false};
  mixed.a = -3;
  mixed.b = true;
  if (mixed.a != -3 || !mixed.b || byte_at(&mixed, 0) != 0x0d) return 2;
  Signs signs = {3, -2, true};
  if (signs.v != 3 || signs.n != -2 || !signs.f || byte_at(&signs, 0) != 0x3b)
    return 3;
  After after;
  after.c = 0x55;
  after.x = 5;
  if (byte_at(&after, 0) != 0x55 || (byte_at(&after, 1) & 15) != 5) return 4;
  Before before;
  before.x = 6;
  before.c = 0x66;
  if ((byte_at(&before, 0) & 15) != 6 || byte_at(&before, 1) != 0x66) return 5;
  Straddle straddle;
  straddle.c = 1;
  straddle.x = 0x12345;
  straddle.d = 2;
  if (straddle.x != 0x12345 || byte_at(&straddle, 1) != 0x45 ||
      byte_at(&straddle, 4) != 2)
    return 6;
  Zero zero = {5, 6};
  if (zero.a != 5 || zero.b != 6 || (byte_at(&zero, 8) & 7) != 6) return 7;
  Unnamed unnamed = {7, 8};
  if (byte_at(&unnamed, 2) != 8) return 8;
  return 0;
}
