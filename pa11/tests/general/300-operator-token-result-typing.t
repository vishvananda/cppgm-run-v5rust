struct Flag
{
  int value;
};

bool operator|(const Flag &lhs, const Flag &rhs)
{
  return lhs.value == 1 && rhs.value == 2;
}

bool operator^(const Flag &lhs, const Flag &rhs)
{
  return lhs.value == 1 && rhs.value == 2;
}

Flag &operator&=(Flag &lhs, const Flag &rhs)
{
  lhs.value = lhs.value + rhs.value;
  return lhs;
}

Flag &operator|=(Flag &lhs, const Flag &rhs)
{
  lhs.value = lhs.value + rhs.value + 1;
  return lhs;
}

Flag &operator^=(Flag &lhs, const Flag &rhs)
{
  lhs.value = lhs.value + rhs.value + 2;
  return lhs;
}

int pick_bool(bool)
{
  return 0;
}

int pick_bool(int)
{
  return 10;
}

int pick_flag(Flag &)
{
  return 0;
}

int pick_flag(int)
{
  return 100;
}

int main()
{
  Flag a;
  Flag b;
  a.value = 1;
  b.value = 2;
  int result = 0;
  result = result + pick_bool(a | b);
  result = result + pick_bool(a ^ b);
  result = result + pick_flag(a &= b);
  result = result + pick_flag(a |= b);
  result = result + pick_flag(a ^= b);
  return result;
}
