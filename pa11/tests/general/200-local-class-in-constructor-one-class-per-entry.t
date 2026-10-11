// A constructor's body declares one local class (9.8) whichever entry runs
// it: the complete object's and a base subobject's.
int calls = 0;

struct counter
{
  int value;

  counter()
  {
    struct step { static int next(int v) { calls = calls + 1; return v + calls; } };
    value = step::next(5);
  }
};

struct derived : counter
{
  derived() {}
};

int main()
{
  counter whole;
  derived part;
  if (whole.value != 6) return 10;
  if (part.value != 7) return 11;
  return calls == 2 ? 0 : 12;
}
