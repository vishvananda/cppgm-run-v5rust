// expect exit 0
int calls;
int receivers;
int number = 7;
struct Left
{
  Left() { ++receivers; }
  ~Left() { --receivers; }
  operator short() { ++calls; return 3; }
};
struct Right
{
  operator long() { ++calls; return 9; }
};
struct Pointer
{
  operator int *() { ++calls; return &number; }
};
struct ConstPointer
{
  operator const int *() { ++calls; return &number; }
};
enum class Mode { first, second };
struct First { operator Mode() { ++calls; return Mode::first; } };
struct Second { operator Mode() { ++calls; return Mode::second; } };
int main()
{
  Right right;
  long left_value = true ? Left() : right;
  if (left_value != 3 || receivers != 0 || calls != 1) return 1;
  long right_value = false ? Left() : right;
  if (right_value != 9 || receivers != 0 || calls != 2) return 2;
  Pointer pointer;
  ConstPointer fixed;
  if ((true ? pointer : fixed) != &number || calls != 3) return 3;
  if ((false ? pointer : fixed) != &number || calls != 4) return 4;
  First first;
  Second second;
  if ((true ? first : second) != Mode::first || calls != 5) return 5;
  if ((false ? first : second) != Mode::second || calls != 6) return 6;
  return receivers != 0;
}
