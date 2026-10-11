struct Value
{
  int x;

  Value() : Value(7) {}
  Value(int v) : x(v) {}
};

int main()
{
  Value v;
  return v.x == 7 ? 0 : 1;
}
