struct Cell {
  int value;

  int get() const
  {
    return 20;
  }

  int get()
  {
    return value;
  }
};

int read_mutable(Cell & cell)
{
  return cell.get();
}

int read_const(const Cell & cell)
{
  return cell.get();
}

int main()
{
  Cell cell = { 7 };
  return read_mutable(cell) * 10 + read_const(cell) == 90 ? 0 : 1;
}
