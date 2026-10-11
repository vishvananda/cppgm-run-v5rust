struct holder
{
  int (value)() noexcept
  {
    return 7;
  }
};

int main()
{
  holder object;
  return object.value() == 7 ? 0 : 1;
}
