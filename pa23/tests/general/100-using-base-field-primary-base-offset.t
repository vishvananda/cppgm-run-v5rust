struct Slot { int value; };
struct Storage { Slot member; };
struct Primary { virtual int anchor() { return 0; } };

struct Base : protected Storage, Primary {
  using Storage::member;
  int read() { return member.value; }
};

struct Derived : Base {};

int main()
{
  Derived value;
  value.member.value = 7;
  if((char *)&value.member == (char *)&value) return 1;
  return value.read() == 7 ? 0 : 2;
}
