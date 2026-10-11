int value;

int main()
{
  auto assign = [=]() { value = 1; };
  assign();
  return value - 1;
}
