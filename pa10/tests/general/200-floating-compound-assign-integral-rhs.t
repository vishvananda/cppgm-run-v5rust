long double scale(unsigned long long den)
{
  long double t = 12.0L;
  t /= den;
  return t;
}

int main()
{
  return scale(3ULL) == 4.0L ? 0 : 1;
}
