// A constructor cannot delegate directly to itself.

struct A
{
  A() : A() {}
};

int main()
{
  A a;
  return 0;
}
