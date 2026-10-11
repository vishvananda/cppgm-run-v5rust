bool both(double lhs, int rhs)
{
  return lhs && rhs;
}

int main()
{
  int guard = 0;
  if(0.0 && ++guard) {
    return 1;
  }
  if(!both(2.0, 1)) {
    return 2;
  }
  return guard == 0 ? 0 : 3;
}
