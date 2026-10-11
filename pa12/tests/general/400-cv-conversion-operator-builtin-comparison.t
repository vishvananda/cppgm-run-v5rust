struct counter
{
  operator long() const volatile
  {
    return 1;
  }

  operator long() const
  {
    return 0;
  }
};

int main()
{
  counter value;
  return value == 0 ? 0 : 1;
}
