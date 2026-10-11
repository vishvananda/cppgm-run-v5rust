namespace detail
{
int operator_arrow_dispatch_(int x)
{
  return x + 2;
}
}

int f()
{
  return detail::operator_arrow_dispatch_(5);
}
