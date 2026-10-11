// An inheriting constructor (12.9p8) whose member's brace-or-equal-
// initializer exits by an exception destroys the members it has already
// constructed by their initializers, in reverse order, then its base
// (15.2p2), and none it has not constructed; a constructor the base runs
// that throws leaves no member to destroy.
int clock;
int trail[8];
int count;

struct Tracked
{
  int id;
  Tracked(int i) : id(i)
  {
    if (i < 0) throw i;
  }
  ~Tracked() { trail[count++] = id; }
};

struct Base
{
  Base(int k)
  {
    if (k == 0) throw 100;
  }
  ~Base() { trail[count++] = 9; }
};

struct Braced : Base { using Base::Base; Tracked a{1}; Tracked b{-2}; Tracked c = 3; };
struct Converted : Base { using Base::Base; Tracked a{4}; Tracked b = 5; Tracked c = -6; Tracked d{7}; };
struct First : Base { using Base::Base; Tracked a{8}; };

int main()
{
  try {
    Braced braced(1);
    return 1;
  } catch (int e) {
    if (e != -2) return 2;
  }
  if (count != 2 || trail[0] != 1 || trail[1] != 9) return 3;
  count = 0;
  try {
    Converted converted(1);
    return 4;
  } catch (int e) {
    if (e != -6) return 5;
  }
  if (count != 3 || trail[0] != 5 || trail[1] != 4 || trail[2] != 9) return 6;
  count = 0;
  try {
    First first(0);
    return 7;
  } catch (int e) {
    if (e != 100) return 8;
  }
  if (count != 0) return 9;
  return 0;
}
