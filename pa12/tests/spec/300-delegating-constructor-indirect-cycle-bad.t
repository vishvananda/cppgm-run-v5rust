// A delegation chain cannot return to an earlier constructor.

struct A
{
  A() : A(1) {}
  A(int) : A() {}
};

int main()
{
  A a;
  return 0;
}
