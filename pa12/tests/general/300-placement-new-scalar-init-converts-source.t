typedef decltype(sizeof(0)) size_t;

void *operator new(size_t, void *p)
{
  return p;
}

int main()
{
  float f = 0.0f;
  int i = 7;
  new (&f) float(i);
  return 0;
}
