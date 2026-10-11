namespace lookup {

struct argument
{
};

struct wrapper
{
  wrapper(const argument &)
  {
  }

  friend wrapper operator+(wrapper, wrapper)
  {
    return wrapper(argument());
  }
};

void probe()
{
  argument value;
  (void)(value + value);
}

}  // namespace lookup

int main()
{
  return 0;
}
