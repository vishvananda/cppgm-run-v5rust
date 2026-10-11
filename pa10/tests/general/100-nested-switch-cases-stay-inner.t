int main()
{
  int outer = 0;
  int inner = 1;
  switch(outer) {
    case 0:
      switch(inner) {
        case 1:
          return 0;
        default:
          return 1;
      }
    default:
      return 2;
  }
  return 3;
}
