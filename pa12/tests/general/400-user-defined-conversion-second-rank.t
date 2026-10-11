struct value_c
{
  operator int() const
  {
    return 2;
  }
};

int pick(bool)
{
  return 1;
}

int pick(short)
{
  return 2;
}

int pick(int)
{
  return 3;
}

int pick(long)
{
  return 4;
}

int main()
{
  value_c value;
  return pick(value) == 3 ? 0 : 1;
}
