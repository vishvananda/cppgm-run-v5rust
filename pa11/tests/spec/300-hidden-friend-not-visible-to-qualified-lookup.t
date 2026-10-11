namespace lookup {

struct wrapper
{
  wrapper()
  {
  }

  friend wrapper operator+(const wrapper &, const wrapper &)
  {
    return wrapper();
  }
};

void probe()
{
  wrapper value;
  (void)lookup::operator+(value, value);
}

}  // namespace lookup

int main()
{
  return 0;
}
