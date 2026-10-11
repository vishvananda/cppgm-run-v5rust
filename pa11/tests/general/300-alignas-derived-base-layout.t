struct Base
{
  char value;
};

struct alignas(16) Derived : Base
{
  char other;
};

int main()
{
  return sizeof(Derived) + alignof(Derived);
}
