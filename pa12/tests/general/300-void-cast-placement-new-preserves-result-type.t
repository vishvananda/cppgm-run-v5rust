void * operator new(unsigned long, void * p)
{
  return p;
}

struct S
{
  int value;

  S(unsigned i) : value(i)
  {
  }
};

void construct(S * p)
{
  (void) new (p) S(16);
}

int main()
{
  S s(1);
  construct(&s);
  return s.value == 16 ? 0 : 1;
}
