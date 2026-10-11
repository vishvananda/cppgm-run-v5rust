int run(int value, bool flag)
{
  while(value) {
    switch(value) {
    case 1:
      if(flag) {
    case 2:
        return 2;
      }
      break;
    default:
      return 3;
    }
    return 4;
  }
  return 0;
}

int main()
{
  return run(2, true) == 2 ? 0 : 1;
}
