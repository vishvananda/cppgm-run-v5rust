int values[3] = {1, 2, 3};
int *cursor = values;

int main()
{
  cursor[1] = 9;
  return values[1] == 9 ? 0 : 1;
}
