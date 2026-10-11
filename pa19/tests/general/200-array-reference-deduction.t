template<typename T, int N>
int length(T (&)[N])
{
  return N;
}

int main()
{
  int values[4] = {0, 1, 2, 3};
  return length(values) == 4 ? 0 : 1;
}
