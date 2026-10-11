#include "100-lazy-static-value-before-shadowing-typedef.h"

int main()
{
  M<int>::n result = 0;
  return result;
}
