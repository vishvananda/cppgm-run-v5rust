struct Table
{
  int values[2];
};

const Table & get_table()
{
  static const Table table = {{1, 2}};
  return table;
}

int read_table()
{
  static const Table & table = get_table();
  return table.values[1];
}

int main()
{
  return read_table() == 2 ? 0 : 1;
}
