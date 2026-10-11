struct cell {
  int first;
  int second;

  cell &operator=(cell const &other)
  {
    first = other.first;
    second = other.second;
    return *this;
  }
};

cell &current(cell *&it)
{
  return *it;
}

void copy_backward_like(cell *first, cell *last, cell *result)
{
  cell *last_iter = last;
  while(first != last_iter) {
    *--result = current(--last_iter);
  }
}

int main()
{
  cell data[4] = {{1, 1}, {2, 2}, {3, 4}, {5, 5}};
  copy_backward_like(data + 1, data + 4, data + 4);
  cell *p = data + 1;
  return p->first == 2 && p->second == 2 ? 0 : 1;
}
