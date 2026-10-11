struct Box
{
  int value;

  friend int get(Box b)
  {
    return b.value;
  }
};

int main()
{
  Box b = {7};
  return get(b) == 7 ? 0 : 1;
}
