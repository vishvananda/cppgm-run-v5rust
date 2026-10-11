int counter = 0;

int init_value()
{
  counter = counter + 1;
  return 7;
}

int g = init_value();
int h = g;

int main()
{
  return counter == 1 && g == 7 && h == 7 ? 0 : 5;
}
