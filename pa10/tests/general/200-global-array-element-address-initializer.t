int values[3];
int *middle = &values[1];

int main()
{
  return middle - values - 1;
}
