template<typename T>
int probe(const T *)
{
  return 1;
}

int main()
{
  int value = 0;
  return probe(&value) == 1 ? 0 : 1;
}
