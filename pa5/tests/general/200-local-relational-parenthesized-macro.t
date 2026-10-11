#define LOW (-2147483647-1)
#define HIGH (2147483647)

bool outside(long disp)
{
  return disp < LOW || disp > HIGH;
}
