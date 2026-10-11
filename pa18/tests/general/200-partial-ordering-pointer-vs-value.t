template<typename T>
int pick(T)
{
  return 1;
}

template<typename T>
int pick(T *)
{
  return 2;
}

int main()
{
  int value = 0;
  return pick(value) == 1 && pick(&value) == 2 ? 0 : 1;
}
