int select_array(const int (&)[3])
{
  return 1;
}

int select_array(const int (&&)[3])
{
  return 2;
}

int main()
{
  int values[3] = { 1, 2, 3 };
  const int const_values[3] = { 4, 5, 6 };

  if(select_array(values) != 1 || select_array(const_values) != 1) {
    return 1;
  }
  return 0;
}
