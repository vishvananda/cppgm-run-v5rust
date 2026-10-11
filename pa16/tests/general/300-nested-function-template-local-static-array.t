template<int N>
int value()
{
  {
    static int data[1] = { N };
    return data[0];
  }
}

int main()
{
  if(value<1>() != 1) return 1;
  if(value<2>() != 2) return 2;
  return 0;
}
