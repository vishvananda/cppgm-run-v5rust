#include "200-included-template-member-dual-capturing-lambda-call.h"

int main()
{
  LambdaBox<int> box;
  return box.run(0) - 1;
}
