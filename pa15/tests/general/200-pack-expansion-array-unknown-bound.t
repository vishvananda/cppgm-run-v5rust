template<int... I>
void run()
{
  int values[] = { I..., 0 };
  (void)values;
}

int main()
{
  run<0, 1>();
  return 0;
}
