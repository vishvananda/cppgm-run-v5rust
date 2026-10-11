struct S;

S * f(bool b, S * p)
{
  return b ? nullptr : p;
}

S * g(bool b, S * p)
{
  return b ? 0 : p;
}
