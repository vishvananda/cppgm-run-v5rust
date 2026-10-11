int main()
{
  int iteration = 0;
  int result = 0;
  while(iteration < 3) {
    iteration = iteration + 1;
    switch(iteration) {
      case 1:
        continue;
      default:
        result = result + 1;
    }
    result = result + 10;
  }
  return result;
}
