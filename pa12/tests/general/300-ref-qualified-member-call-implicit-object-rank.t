struct RefPick {
  int pick() &
  {
    return 1;
  }

  int pick() const &
  {
    return 2;
  }

  int pick() &&
  {
    return 3;
  }
};

RefPick make_ref_pick()
{
  return RefPick();
}

int main()
{
  RefPick value;
  const RefPick const_value;
  return value.pick() * 100 + const_value.pick() * 10 + make_ref_pick().pick() == 123 ? 0 : 1;
}
