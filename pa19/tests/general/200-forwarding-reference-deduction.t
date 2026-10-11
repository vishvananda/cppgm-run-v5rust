#include "support.h"

template<typename T>
struct category;

template<>
struct category<int &>
{
  static int value() { return 1; }
};

template<>
struct category<int>
{
  static int value() { return 2; }
};

template<typename T>
int classify(T &&)
{
  return category<T>::value();
}

int main()
{
  int x = 0;
  return classify(x) == 1 && classify(0) == 2 ? 0 : 1;
}
