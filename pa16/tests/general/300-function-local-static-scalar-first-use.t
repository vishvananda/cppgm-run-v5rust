int counter = 0;

int init_value()
{
  counter = counter + 1;
  return 7;
}

int & get_value()
{
  static int value = init_value();
  return value;
}

int main()
{
  if(get_value() != 7) {
    return 10;
  }
  if(get_value() != 7) {
    return 11;
  }
  return counter == 1 ? 0 : counter;
}
