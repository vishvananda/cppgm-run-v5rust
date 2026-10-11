// N3485 focus: 4.10 [conv.ptr] null pointer conversion from nullptr
int f(int *p) {
  int *q = nullptr;
  if (p == nullptr)
    return q == p;
  return q != p;
}
