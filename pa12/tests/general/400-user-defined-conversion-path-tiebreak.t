struct stream
{
  int value;
};

struct source
{
  operator int() const
  {
    return 1;
  }
};

struct fallback_lhs
{
  fallback_lhs(stream &)
  {
  }
};

struct fallback_rhs
{
  fallback_rhs(const source &)
  {
  }
};

int pick(stream &, bool)
{
  return 0;
}

int pick(fallback_lhs, fallback_rhs)
{
  return 1;
}

int main()
{
  stream lhs = {0};
  source rhs;
  return pick(lhs, rhs);
}
