struct Cell {
  int get()
  {
    return 1;
  }
};

int main()
{
  const Cell cell = {};
  return cell.get();
}
