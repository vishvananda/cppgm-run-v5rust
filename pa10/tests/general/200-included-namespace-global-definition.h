namespace inc {
volatile unsigned long counter = 0;

int touch()
{
  counter = 7;
  return counter == 7 ? 0 : 1;
}
}
