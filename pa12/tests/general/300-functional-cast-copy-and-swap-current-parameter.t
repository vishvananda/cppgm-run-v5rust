struct value
{
  long data;

  value(long input) : data(input) {}
  value(const value &other) : data(other.data) {}

  void swap(value &other)
  {
    long saved = data;
    data = other.data;
    other.data = saved;
  }

  value &operator=(const value &other)
  {
    value(other).swap(*this);
    return *this;
  }
};

int main()
{
  value left(1);
  value right(2);
  left = right;
  return left.data == 2 ? 0 : 1;
}
