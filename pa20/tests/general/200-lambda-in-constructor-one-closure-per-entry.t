// A lambda in a constructor's body is one closure type (5.1.2p3) whichever
// entry runs the body: the complete object's and a base subobject's call the
// same function through its pointer.
int (*seen[2])() = { 0, 0 };

struct counter
{
  int value;

  explicit counter(int slot)
  {
    auto step = []() -> int { return 4; };
    int (*pointer)() = step;
    seen[slot] = pointer;
    value = step();
  }
};

struct derived : counter
{
  derived() : counter(1) {}
};

int main()
{
  counter whole(0);
  derived part;
  if (seen[0] != seen[1]) return 10;
  return whole.value + part.value == 8 ? 0 : 11;
}
