int main()
{
  unsigned n = 3;
  int count = 0;

  for(; n > 0; count = count + 1, (void)--n) {
  }

  return count == 3 && n == 0 ? 0 : 1;
}
