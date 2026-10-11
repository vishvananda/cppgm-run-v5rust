enum Op
{
  Plus = 4,
  Minus = 6,
  Bang = 10
};

int f(Op op)
{
  if(op == Minus)
    return 1;
  if(op == Plus)
    return 2;
  if(op == Bang)
    return 3;
  return 0;
}

int main()
{
  return 0;
}
